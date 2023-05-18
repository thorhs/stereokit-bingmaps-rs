use glam::{Vec2, Vec3};

use crate::bing_api::BoundingBox;

pub const EARTH_CIRCUMFERENCE: f64 = 40075040.0f64;
pub const EARTH_TALLEST: f32 = 8848.0f32;

pub fn dist_latitude<T: num::Float>(a: T, b: T) -> T {
    ((a - b) * num::NumCast::from(EARTH_CIRCUMFERENCE).unwrap())
        / num::NumCast::from(360.0).unwrap()
}

pub fn dist_longitude<T: num::Float>(a: T, b: T, latitude_y: T) -> T {
    ((a - b) * num::NumCast::from(EARTH_CIRCUMFERENCE).unwrap() * latitude_y.to_radians().cos())
        / num::NumCast::from(360.0).unwrap()
}

pub fn lat_lon_bounds(latitude_y: f64, longitude_x: f64, radius_m: f64) -> BoundingBox {
    let radius_y = (radius_m * 360.0) / EARTH_CIRCUMFERENCE;
    let radius_x = (radius_m * 360.0) / (EARTH_CIRCUMFERENCE * latitude_y.to_radians().cos());

    BoundingBox {
        south_latitude: latitude_y - radius_y,
        west_longitude: longitude_x - radius_x,
        north_latitude: latitude_y + radius_y,
        east_longitude: longitude_x + radius_x,
    }
}

pub fn bounds_size(bounds: BoundingBox) -> Vec2 {
    Vec2::from_array([
        dist_longitude(
            bounds.east_longitude,
            bounds.west_longitude,
            (bounds.north_latitude + bounds.south_latitude) / 2.0,
        ) as f32,
        dist_latitude(bounds.north_latitude, bounds.south_latitude) as f32,
    ])
}

pub fn bounds_to_world(query_box: BoundingBox, given_box: BoundingBox) -> (Vec3, Vec2) {
    let query_center = query_box.center();
    let given_center = given_box.center();

    let offset = Vec2 {
        x: dist_longitude(given_center.x, query_center.x, query_center.y),
        y: dist_latitude(given_center.y as f64, query_center.y as f64) as f32,
    };

    let bounds = bounds_size(given_box);

    let size = Vec3 {
        x: bounds.x,
        z: bounds.y,
        y: EARTH_TALLEST,
    };

    (size, offset)
}

pub fn elevation_relative_height(input: &[i64]) -> Vec<f32> {
    input.iter().map(|&i| (i as f32) / EARTH_TALLEST).collect()
}
