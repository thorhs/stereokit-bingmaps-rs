use glam::{Vec2, Vec3};

#[derive(Debug)]
pub struct WorldCalculator {
    pub real_center: Vec2,
    pub real_size: Vec2,
    pub radius: f32,
    pub depth_scaler: f32,
    pub magnitude_scaler: f32,
}

impl WorldCalculator {
    pub fn new(
        world_bounds: crate::bing_api::BoundingBox,
        radius: f32,
        depth_scaler: f32,
        magnitude_scaler: f32,
    ) -> WorldCalculator {
        WorldCalculator {
            real_center: world_bounds.center(),
            real_size: world_bounds.size(),
            radius,
            depth_scaler,
            magnitude_scaler,
        }
    }

    pub fn project_real(&self, real: Vec2, depth: f32) -> Vec3 {
        let mut offset = dbg!(dbg!(self.real_center) - dbg!(real));
        offset.x = -offset.x;

        let real_scaler = Vec3::new(
            self.radius / self.real_size.x / 2.0,
            self.depth_scaler,
            self.radius / self.real_size.y / 2.0,
        );

        Vec3::new(offset.x, depth, offset.y) * real_scaler
    }
}
