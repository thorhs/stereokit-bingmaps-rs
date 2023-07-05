pub const BING_KEY: &str = include_str!("../bing_api.key");

use anyhow::{Context, Result};
use reqwest::Client;

use crate::bing_api::BoundingBox;

use glam::{Vec2, Vec3};

#[allow(dead_code)]
#[derive(derive_more::Display, Clone, Debug, Hash, PartialEq, Eq, Copy)]
pub enum ImageryType {
    Aerial,
    AerialWithLabels,
    AerialWithLabelsOnDemand,
    Streetside,
    BirdsEye,
    BirdsEyeWithLabels,
    Road,
    CanvasDark,
    CanvasLight,
    CanvasGray,
}

pub struct BingSession {
    bing_api_key: String,
    client: Client,
}

pub enum RequestType {
    ImageData,
    Metadata,
}

impl RequestType {
    pub fn to_id(&self) -> u8 {
        match self {
            RequestType::ImageData => 0,
            RequestType::Metadata => 1,
        }
    }
}

impl BingSession {
    pub fn new() -> BingSession {
        BingSession {
            bing_api_key: BING_KEY.to_owned(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn build_image_request(
        &self,
        imagery_type: &ImageryType,
        region_bounds: &BoundingBox,
        request_type: RequestType,
    ) -> Result<reqwest::Request> {
        let url = format!("https://dev.virtualearth.net/REST/v1/Imagery/Map/{}?&format=png&mapArea={}&mapMetadata={}&key={}", imagery_type, region_bounds, request_type.to_id(), self.bing_api_key);

        dbg!(&url);

        self.client.get(url).build().context("Building request")
    }

    pub async fn request_color_image(
        &self,
        imagery_type: &ImageryType,
        region_bounds: &BoundingBox,
    ) -> Result<bytes::Bytes> {
        let req = self
            .build_image_request(imagery_type, region_bounds, RequestType::ImageData)
            .await?;

        let res = self.client.execute(req).await?;

        if let Some(content_type) = res.headers().get("content-type") {
            if content_type != "image/png" {
                anyhow::bail!("Image type {} not supported", content_type.to_str()?);
            }
        }

        let image_bytes = res.bytes().await?;

        // let tex = Tex::DEFAULT;
        // let size = Vec3::from_array([0.0, 0.0, 0.0]);
        // let center = Vec2::from_array([0.0, 0.0]);
        Ok(image_bytes)
    }

    pub async fn request_metadata(
        &self,
        imagery_type: &ImageryType,
        region_bounds: &BoundingBox,
    ) -> Result<(/*Tex, */ Vec3, Vec2)> {
        let req = self
            .build_image_request(imagery_type, region_bounds, RequestType::Metadata)
            .await?;

        let res = self.client.execute(req).await?;

        let metadata_response: Result<crate::bing_api::ImgMetadataResponse> =
            res.json().await.context("Error unmarshaling metadata json");

        let metadata_response = metadata_response?;

        if metadata_response.resourceSets.len() != 1 {
            anyhow::bail!("Unexpected number of resource sets returned when fetching metadata data, expected 1 got {}", metadata_response.resourceSets.len());
        }

        let metadata_sets = metadata_response.resourceSets.get(0).unwrap();

        if metadata_sets.resources.len() != 1 {
            anyhow::bail!("Unexpected number of resources returned when fetching metadata data, expected 1 got {}", metadata_sets.resources.len());
        }

        let metadata_set = metadata_sets.resources.get(0).unwrap();

        let bbox = metadata_set.bbox.clone();
        let region_bounds: BoundingBox = dbg!(bbox).try_into()?;

        let (size, center) = crate::geo::bounds_to_world(region_bounds.clone(), region_bounds);
        dbg!((size, center));

        Ok((size, center))
    }

    pub async fn request_image_and_data(
        &self,
        imagery_type: &ImageryType,
        region_bounds: &BoundingBox,
    ) -> Result<(bytes::Bytes, Vec3, Vec2)> {
        let (size, center) = self.request_metadata(imagery_type, region_bounds).await?;
        let texture_bytes = self
            .request_color_image(imagery_type, region_bounds)
            .await?;

        Ok((texture_bytes, size, center))
    }

    pub async fn request_elevation(
        &self,
        region_bounds: &BoundingBox,
    ) -> Result<(Vec<f32>, Vec3, Vec2)> {
        let url = format!(
            "http://dev.virtualearth.net/REST/v1/Elevation/Bounds?bounds={}&rows=32&cols=32&key={}",
            region_bounds, self.bing_api_key
        );

        dbg!(&url);

        let req = self.client.get(url).build().context("Building request")?;

        let res = self.client.execute(req).await?;

        let elevation_response: Result<crate::bing_api::ElevationResponse> = res
            .json()
            .await
            .context("Error unmarshaling elevation json");

        let elevation_response = elevation_response?;

        if elevation_response.resourceSets.len() != 1 {
            anyhow::bail!("Unexpected number of resource sets returned when fetching elevation data, expected 1 got {}", elevation_response.resourceSets.len());
        }

        let elevation_sets = elevation_response.resourceSets.get(0).unwrap();

        if elevation_sets.resources.len() != 1 {
            anyhow::bail!("Unexpected number of resources returned when fetching elevation data, expected 1 got {}", elevation_sets.resources.len());
        }

        let elevations = elevation_sets.resources.get(0).unwrap();

        let relative_heights = crate::geo::elevation_relative_height(&elevations.elevations);

        let (size, center) =
            crate::geo::bounds_to_world(region_bounds.to_owned(), region_bounds.to_owned());

        Ok((relative_heights, size, center))
    }
}
