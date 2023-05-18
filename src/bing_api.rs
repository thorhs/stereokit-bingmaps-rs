#![allow(dead_code, non_snake_case)]

use std::fmt::Display;

use glam::Vec2;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Point {
    pub(crate) coordinates: Vec<String>,
}
#[derive(Deserialize)]
pub(crate) struct ImageResponseResource {
    pub(crate) __type: String,
    pub(crate) bbox: BoundingBox,
    pub(crate) imageHeight: String,
    pub(crate) imageWidth: String,
    pub(crate) mapCenter: Point,
    pub(crate) pushpins: Vec<()>,
    pub(crate) zoom: String,
}
#[derive(Deserialize)]
pub(crate) struct ImageResponseResourceSet {
    pub(crate) estimatedTotal: i64,
    pub(crate) resources: Vec<ImageResponseResource>,
}
#[derive(Deserialize)]
pub(crate) struct ImageryResponse {
    pub(crate) authenticationResultCode: String,
    pub(crate) brandLogoUri: String,
    pub(crate) copyright: String,
    pub(crate) resourceSets: Vec<ImageResponseResourceSet>,
    pub(crate) statusCode: i64,
    pub(crate) statusDescription: String,
    pub(crate) traceId: String,
}

#[derive(Deserialize)]
pub(crate) struct ImgMetadataResponseResource {
    pub(crate) __type: String,
    pub(crate) bbox: Vec<f64>,
    pub(crate) imageHeight: String,
    pub(crate) imageWidth: String,
    pub(crate) mapCenter: Point,
    pub(crate) pushpins: Vec<()>,
    pub(crate) zoom: String,
}
#[derive(Deserialize)]
pub(crate) struct ImgMetadataResponseResourceSet {
    pub(crate) estimatedTotal: i64,
    pub(crate) resources: Vec<ImgMetadataResponseResource>,
}
#[derive(Deserialize)]
pub(crate) struct ImgMetadataResponse {
    pub(crate) authenticationResultCode: String,
    pub(crate) brandLogoUri: String,
    pub(crate) copyright: String,
    pub(crate) resourceSets: Vec<ImgMetadataResponseResourceSet>,
    pub(crate) statusCode: i64,
    pub(crate) statusDescription: String,
    pub(crate) traceId: String,
}

#[derive(Deserialize)]
pub(crate) struct ElevationResponseResource {
    pub(crate) __type: String,
    pub(crate) elevations: Vec<i64>,
    pub(crate) zoomLevel: i64,
}
#[derive(Deserialize)]
pub(crate) struct ElevationResponseResourceSet {
    pub(crate) estimatedTotal: i64,
    pub(crate) resources: Vec<ElevationResponseResource>,
}
#[derive(Deserialize)]
pub(crate) struct ElevationResponse {
    pub(crate) authenticationResultCode: String,
    pub(crate) brandLogoUri: String,
    pub(crate) copyright: String,
    pub(crate) resourceSets: Vec<ElevationResponseResourceSet>,
    pub(crate) statusCode: i64,
    pub(crate) statusDescription: String,
    pub(crate) traceId: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BoundingBox {
    pub east_longitude: f64,
    pub west_longitude: f64,
    pub north_latitude: f64,
    pub south_latitude: f64,
}

impl BoundingBox {
    pub fn center(&self) -> Vec2 {
        Vec2 {
            x: ((self.west_longitude + self.east_longitude) / 2.0) as f32,
            y: ((self.north_latitude + self.south_latitude) / 2.0) as f32,
        }
    }
}

impl Display for BoundingBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{}",
            self.south_latitude, self.west_longitude, self.north_latitude, self.east_longitude
        )
    }
}

impl TryFrom<Vec<f64>> for BoundingBox {
    type Error = anyhow::Error;

    fn try_from(value: Vec<f64>) -> Result<Self, Self::Error> {
        if value.len() != 4 {
            anyhow::bail!("Invalid number of input elements, should be 4");
        }

        Ok(BoundingBox {
            south_latitude: *value.get(0).unwrap(),
            west_longitude: *value.get(1).unwrap(),
            north_latitude: *value.get(2).unwrap(),
            east_longitude: *value.get(3).unwrap(),
        })
    }
}
