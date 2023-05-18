use glam::{Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};
use stereokit::*;

#[derive(Debug)]
pub struct Chunk {
    pub center_offset: Vec3,
    pub transform: Mat4,
}

pub struct Terrain {
    pub chunks: Vec<Chunk>,
    pub chunk_center: Vec3,
    pub chunk_size: f32,
    pub mesh: Mesh,
    local_position: Vec3,
    pub heightmap_start: Vec2,
    pub heightmap_size: Vec3,
    pub colormap_start: Vec2,
    pub colormap_size: Vec2,
    pub material: Material,
    pub clip_radius: f32,
}

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

impl Terrain {
    pub fn get_local_position(&self) -> Vec3 {
        self.local_position
    }
    pub fn set_local_position(&mut self, position: Vec3) {
        self.local_position = position;

        self.update_chunks();
    }

    pub fn new(
        sk: &stereokit::SkSingle,
        chunk_detail: i32,
        chunk_size: f32,
        chunk_grid: i32,
    ) -> anyhow::Result<Terrain> {
        let material = sk.material_create(sk.shader_create_file("terrain.hlsl")?);
        let mesh = sk.mesh_gen_plane(
            Vec2::ONE * chunk_size,
            VEC3_UP,
            VEC3_FORWARD,
            chunk_detail,
            true,
        );

        let mut chunks = Vec::with_capacity((chunk_grid * chunk_grid) as usize);

        let half = (chunk_grid as f32 / 2.0) - 0.5;
        for y in 0..chunk_grid {
            for x in 0..chunk_grid {
                let pos = Vec3::new(x as f32 - half, 0.0, y as f32 - half) * chunk_size;
                chunks.push(Chunk {
                    center_offset: pos,
                    transform: Mat4::ZERO,
                });
            }
        }

        Ok(Terrain {
            chunks,
            chunk_center: Vec3::ZERO,
            chunk_size,
            mesh,
            local_position: Vec3::ZERO,
            heightmap_start: Default::default(),
            heightmap_size: Default::default(),
            colormap_start: Default::default(),
            colormap_size: Default::default(),
            material,
            clip_radius: CLIP_RADIUS,
        })
    }

    pub fn set_heightmap_data<T: StereoKitMultiThread>(
        &mut self,
        sk: &T,
        height_data: impl AsRef<Tex>,
        height_dimensions: Vec3,
        height_center: Vec2,
    ) {
        self.set_heightmap_dimensions(height_dimensions, height_center);
        sk.material_set_texture(&self.material, "world", &height_data);
    }

    pub fn set_heightmap_dimensions(&mut self, height_dimensions: Vec3, height_center: Vec2) {
        self.heightmap_start = height_center
            - Vec2 {
                x: height_dimensions.x / 2.0,
                y: height_dimensions.z / 2.0,
            };
        self.heightmap_size = height_dimensions;
    }

    pub fn set_colormap_data<T: StereoKitMultiThread>(
        &mut self,
        sk: &T,
        color_data: impl AsRef<Tex>,
        color_dimensions: Vec2,
        color_center: Vec2,
    ) {
        self.set_colormap_dimensions(color_dimensions, color_center);
        sk.material_set_texture(&self.material, "world_color", &color_data);
    }

    pub fn set_colormap_dimensions(&mut self, color_dimensions: Vec2, color_center: Vec2) {
        self.colormap_start = color_center - color_dimensions / 2.0;
        self.colormap_size = color_dimensions;
    }

    pub fn update_chunks(&mut self) {
        self.chunks.iter_mut().for_each(|c| {
            c.transform =
                Mat4::from_translation(c.center_offset + self.chunk_center + self.local_position)
        })
    }

    pub fn update<T: StereoKitDraw>(&mut self, sk: &T) {
        let offset = self.chunk_center + self.local_position;
        let mut update = false;

        if offset.x > self.chunk_size * 0.4 {
            self.chunk_center.x -= self.chunk_size * 0.5;
            update = true;
        } else if offset.x < self.chunk_size * -0.4 {
            self.chunk_center.x += self.chunk_size * 0.5;
            update = true;
        }

        if offset.z > self.chunk_size * 0.4 {
            self.chunk_center.z -= self.chunk_size * 0.5;
            update = true;
        } else if offset.z < self.chunk_size * -0.4 {
            self.chunk_center.z += self.chunk_size * 0.5;
            update = true;
        }

        if update {
            self.update_chunks();
        }

        sk.material_set_vector4(
            &self.material,
            "world_size",
            Vec4::from((
                sk.hierarchy_to_world_point(self.local_position + self.heightmap_start.x0y())
                    .xz(),
                sk.hierarchy_to_world_direction(self.heightmap_size.x0z())
                    .xz(),
            )),
        );

        sk.material_set_vector4(
            &self.material,
            "color_size",
            Vec4::from((
                sk.hierarchy_to_world_point(self.local_position + self.colormap_start.x0y())
                    .xz(),
                sk.hierarchy_to_world_direction(self.colormap_size.x0y())
                    .xz(),
            )),
        );

        let sizes =
            sk.hierarchy_to_world_direction(Vec3::new(CLIP_RADIUS, self.heightmap_size.y, 0.0));
        let clip_center_shader = sk.hierarchy_to_world_point(Vec3::ZERO);

        sk.material_set_vector4(
            &self.material,
            "clip_vars",
            Vec4::new(
                clip_center_shader.x,
                clip_center_shader.y,
                clip_center_shader.z,
                sizes.x * sizes.x,
            ),
        );

        sk.material_set_float(&self.material, "world_height", sizes.y);

        self.chunks.iter().for_each(|c| {
            sk.render_add_mesh(
                &self.mesh,
                &self.material,
                c.transform,
                named_colors::WHITE,
                RenderLayer::LAYER0,
            )
        });
    }
}

pub trait Vec2Stuff {
    fn x0y(self) -> Vec3;
    fn angle(self) -> f32;
}

impl Vec2Stuff for Vec2 {
    fn x0y(self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.y)
    }
    fn angle(self) -> f32 {
        let mut result = self.y.atan2(self.x).to_degrees();
        if result.is_sign_negative() {
            result += 360.0;
        }

        result
    }
}

pub trait Vec3Stuff {
    fn x0z(self) -> Vec3;
}

impl Vec3Stuff for Vec3 {
    fn x0z(self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.z)
    }
}
