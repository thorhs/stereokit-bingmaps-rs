use anyhow::{Context, Result};
use geo::Polygon;
use glam::{Mat4, Quat, Vec2, Vec3};
use reqwest::Client;
use stereokit::*;

use crate::world_calculator::*;
use skjalftalisa::response::Quake;

pub struct Quakes {
    pub quakes: Vec<Quake>,
    pub projected_quakes: Vec<ProjectedQuake>,
    pub world_calculator: WorldCalculator,
    pub mesh: Mesh,
    local_position: Vec3,
    pub material: Material,
    pub clip_radius: f32,
}

pub struct ProjectedQuake {
    pos: Vec3,
    magnitude: f32,
    time: i64,
}

static MAGNITUDE_SCALE: f32 = 1.0f32;
static CLIP_RADIUS: f32 = 0.5f32;
static VEC3_UP: Vec3 = Vec3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};
static VEC3_FORWARD: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: -1.0,
};

impl Quakes {
    pub fn get_local_position(&self) -> Vec3 {
        self.local_position
    }
    pub fn set_local_position(&mut self, position: Vec3) {
        self.local_position = position;
    }

    pub fn new(sk: &stereokit::SkSingle) -> anyhow::Result<Quakes> {
        let material = sk.material_copy(Material::DEFAULT);
        let mesh = sk.mesh_copy(Mesh::SPHERE);

        Ok(Quakes {
            quakes: Vec::new(),
            projected_quakes: Vec::new(),
            world_calculator: WorldCalculator::new(
                crate::bing_api::BoundingBox::default(),
                0.0,
                0.0,
                0.0,
            ),
            mesh,
            local_position: Vec3::ZERO,
            material,
            clip_radius: CLIP_RADIUS,
        })
    }

    pub fn set_quakes(&mut self, quakes: Vec<Quake>) {
        self.quakes = quakes;

        self.reproject();
    }

    pub fn set_world_calculator(&mut self, world_calculator: WorldCalculator) {
        self.world_calculator = world_calculator;
    }

    pub fn reproject(&mut self) {
        self.projected_quakes = self
            .quakes
            .iter()
            .map(|q| ProjectedQuake {
                pos: dbg!(&self.world_calculator).project_real(
                    Vec2::new(q.long as f32, q.lat as f32),
                    (-q.depth) as f32 * self.world_calculator.depth_scaler,
                ),
                magnitude: q.magnitude as f32 * self.world_calculator.magnitude_scaler,
                time: q.time,
            })
            .collect();
    }

    pub fn update<T: StereoKitDraw>(&mut self, sk: &T) {
        self.projected_quakes.iter().for_each(|q| {
            sk.render_add_mesh(
                &self.mesh,
                &self.material,
                /* Mat4::from_translation(
                    Vec3::new(q.long as f32, q.lat as f32, 0.0) + self.local_position,
                ), */
                Mat4::from_scale_rotation_translation(
                    Vec3::ONE * q.magnitude,
                    Quat::IDENTITY,
                    q.pos,
                ),
                named_colors::RED,
                RenderLayer::LAYER0,
            )
        });
    }
}

fn quake_to_transform(quake: &Quake) -> Mat4 {
    Mat4::ZERO
}
