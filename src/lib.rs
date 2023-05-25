use glam::{Mat4, Quat, Vec2, Vec3, Vec3Swizzles};
use std::collections::hash_map::HashMap;
use stereokit::*;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::bing::BingSession;

use tokio::runtime::Runtime;

mod bing;
mod bing_api;
mod geo;
mod terrain;
mod ui;

use bing_api::BoundingBox;
use terrain::*;

use skjalftalisa::{get_quakes, request::SkjalftalisaRequest, response::SkjalftalisaResponse};

/*
#[derive(Debug)]
struct ImgRequest {
    imagery_type: bing::ImageryType,
    region_bounds: BoundingBox,
}

#[derive(Debug)]
struct ElevRequest {
    region_bounds: BoundingBox,
}
*/

static VEC3_UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);
static VEC3_FORWARD: Vec3 = Vec3::new(0.0, 0.0, -1.0);

#[derive(Debug)]
enum BingRequest {
    Location(bing::ImageryType, usize),
    Quakes(SkjalftalisaRequest),
}

#[derive(Debug, Clone)]
enum BingResponse {
    Location(((bytes::Bytes, Vec3, Vec2), (Vec<f32>, Vec3, Vec2))),
    Quakes(SkjalftalisaResponse),
}

async fn bing_fetch_loop(
    session: BingSession,
    request_rx: &mut Receiver<BingRequest>,
    result_tx: &Sender<BingResponse>,
) {
    let mut cache: HashMap<(bing::ImageryType, usize), BingResponse> = HashMap::new();
    while let Some(req) = request_rx.recv().await {
        match req {
            BingRequest::Location(img_type, location_id) => {
                dbg!("Requesting {} at location {}", &img_type, &location_id);
                if let Some(resp) = cache.get(&(img_type, location_id)) {
                    result_tx
                        .send(resp.to_owned())
                        .await
                        .expect("Error sending cached texture response");
                    continue;
                }
                let location = LOCATIONS.get(location_id).expect("Location not found");
                let img_resp = session
                    .request_image_and_data(&img_type, location)
                    .await
                    .expect("Error retreiving bing image data");

                let elev_resp = session
                    .request_elevation(location)
                    .await
                    .expect("Error retreiving bing image data");

                let resp = BingResponse::Location((img_resp, elev_resp));
                cache.insert((img_type, location_id), resp.clone());
                result_tx
                    .send(resp)
                    .await
                    .expect("Error sending bing texture response");
            }
            BingRequest::Quakes(_req) => {
                let resp = skjalftalisa::response::demo_data().expect("Unable to get demo data");

                result_tx
                    .send(BingResponse::Quakes(resp))
                    .await
                    .expect("Error sending quakes response");
            }
        }
    }

    println!("Fetch loop exited");
}

#[cfg_attr(
    target_os = "android",
    ndk_glue::main(backtrace = "on", logger(level = "debug", tag = "my-tag"))
)]
pub fn main() {
    println!("Hello World");

    let session = BingSession::new();

    let rt = Runtime::new().unwrap();

    let (req_tx, mut req_rx) = tokio::sync::mpsc::channel(10);
    let (resp_tx, resp_rx) = tokio::sync::mpsc::channel(10);

    rt.spawn(async move { bing_fetch_loop(session, &mut req_rx, &resp_tx).await });

    let mut sk = SettingsBuilder::new()
        .assets_folder("assets")
        .display_preference(DisplayMode::Flatscreen)
        .disable_flatscreen_mr_sim(false)
        .no_flatscreen_fallback(true)
        .init()
        .unwrap();

    let main = Main::new(&mut sk, req_tx, resp_rx).unwrap();

    main._main(sk).unwrap();
}

static LOCATIONS: once_cell::sync::Lazy<Vec<BoundingBox>> = once_cell::sync::Lazy::new(|| {
    vec![
        geo::lat_lon_bounds(22.0, -159.5, 20000.0),
        geo::lat_lon_bounds(36.3, -112.75, 10000.0),
        geo::lat_lon_bounds(27.98, 86.92, 10000.0),
        geo::lat_lon_bounds(-13.16, -72.54, 10000.0),
    ]
});

struct Main {
    terrain_scale: f32,
    terrain: Terrain,
    drag_active: bool,
    req_tx: Sender<BingRequest>,
    resp_rx: Receiver<BingResponse>,
    location_id: Option<usize>,
    pedestal_model: Model,
    compass_model: Model,
    widget_model: Model,
    drag_start: Vec3,
    drag_widget_start: Vec3,
    terrain_pose: Pose,
    map_height_size: Vec3,
    map_height_center: Vec2,
    map_color_size: Vec3,
    map_color_center: Vec2,
    ui_angle: f32,
}

impl Main {
    fn new(
        sk: &mut SkSingle,
        req_tx: Sender<BingRequest>,
        resp_rx: Receiver<BingResponse>,
    ) -> anyhow::Result<Main> {
        Ok(Main {
            terrain_scale: 0.00004f32,
            terrain: Terrain::new(sk, 64, 0.6, 2).unwrap(),
            drag_active: false,
            req_tx,
            resp_rx,
            location_id: None,
            pedestal_model: sk.model_create_file("Pedestal.glb", Some(Shader::UI))?,
            compass_model: sk.model_create_file("Compass.glb", None::<Shader>)?,
            widget_model: sk.model_create_file("MoveWidget.glb", None::<Shader>)?,
            drag_start: Default::default(),
            drag_widget_start: Default::default(),
            terrain_pose: Default::default(),
            map_height_size: Default::default(),
            map_height_center: Default::default(),
            map_color_size: Default::default(),
            map_color_center: Default::default(),
            ui_angle: 0.0,
        })
    }

    fn load_location<T: StereoKitSingleThread>(
        &mut self,
        sk: &T,
        location_id: Option<usize>,
    ) -> anyhow::Result<()> {
        dbg!(location_id);
        if location_id == self.location_id {
            return Ok(());
        }
        self.location_id = location_id;

        let terrain: &mut Terrain = &mut self.terrain;

        terrain.set_colormap_data(sk, Tex::DEFAULT, Vec2::ZERO, Vec2::ZERO);
        terrain.set_heightmap_data(sk, Tex::BLACK, Vec3::ZERO, Vec2::ZERO);
        terrain.set_local_position(Vec3::ZERO);
        terrain.update_chunks();

        self.req_tx
            .blocking_send(BingRequest::Location(
                bing::ImageryType::Aerial,
                location_id.unwrap(),
            ))
            .expect("Unable to send bing request");

        if let BingResponse::Location((
            (tex_bytes, tex_size, tex_center),
            (height_data, height_size, height_center),
        )) = self
            .resp_rx
            .blocking_recv()
            .expect("Unable to fetch location data")
        {
            terrain.set_colormap_data(
                sk,
                sk.tex_create_mem(&tex_bytes, true, 0)?,
                tex_size.xz() * self.terrain_scale,
                tex_center * self.terrain_scale,
            );

            self.map_color_center = tex_center;
            self.map_color_size = tex_size;

            let mut relative_height_colors: Vec<Color128> =
                Vec::with_capacity(height_data.len() * 4);
            height_data
                .into_iter()
                .for_each(|h| relative_height_colors.push(named_colors::WHITE * h));

            let height_tex = sk.tex_create(TextureType::IMAGE_NO_MIPS, TextureFormat::RGBA128);
            sk.tex_set_colors(
                &height_tex,
                32,
                32,
                TextureFormat::RGBA128,
                &relative_height_colors,
            );

            terrain.set_heightmap_data(
                sk,
                height_tex,
                height_size * self.terrain_scale,
                height_center * self.terrain_scale,
            );

            self.map_height_center = height_center;
            self.map_height_size = height_size;
        } else {
            anyhow::bail!("Unable to fetch location data");
        }

        Ok(())
    }

    fn show_terrain<T: StereoKitDraw>(&mut self, sk: &T) -> anyhow::Result<()> {
        let hand = sk.input_hand(Handed::Right);

        let widget_pos = sk.hierarchy_to_local_point(
            hand.fingers[ui::FingerId::Index as usize][ui::JointId::Tip as usize].position * 0.5
                + hand.fingers[ui::FingerId::Thumb as usize][ui::JointId::Tip as usize].position
                    * 0.5,
        );

        let hand_in_volume =
            widget_pos.y > 0.0 && widget_pos.xz().length() < self.terrain.clip_radius;

        if self.drag_active || hand_in_volume {
            let active_mod = match self.drag_active {
                true => 1.5f32,
                false => 1.0f32,
            };

            sk.render_add_model(
                &self.widget_model,
                Mat4::from_translation(widget_pos).mul_scalar(active_mod),
                named_colors::WHITE * active_mod,
                RenderLayer::LAYER0,
            );

            let hand_interacting =
                unsafe { stereokit_sys::ui_is_interacting(Handed::Right as u32) == 0 };

            if hand_interacting && hand.pinch_state == ButtonState::JUST_ACTIVE {
                self.drag_start = self.terrain.get_local_position();
                self.drag_widget_start = widget_pos;
                self.drag_active = true;
            }

            if self.drag_active && hand.pinch_state == ButtonState::ACTIVE {
                let mut new_pos = self.drag_start + (widget_pos + self.drag_widget_start);
                new_pos.y = 0.0;
                self.terrain.set_local_position(new_pos);
            }
        }

        if hand.pinch_state == ButtonState::JUST_INACTIVE {
            self.drag_active = false;
        }

        self.terrain.update(sk);
        Ok(())
    }

    fn show_terrain_widget<T: StereoKitDraw>(&mut self, sk: &T) -> anyhow::Result<()> {
        // Create an affordance for the pedestal that the terrain and UI will
        // rest on. The user can drag this around the environment, but it
        // doesn't rotate at all. The pedestal model asset has a diameter of
        // 1, or radius of 0.5, so the proper scale is radius * 2!
        let pedestal_scale = self.terrain.clip_radius * 2.0;

        ui::ui_handle_begin(
            "TerrainWidget",
            &mut self.terrain_pose,
            sk.model_get_bounds(&self.pedestal_model),
        );

        sk.render_add_model(
            &self.pedestal_model,
            Mat4::from_scale(Vec3::ONE * pedestal_scale),
            named_colors::WHITE,
            RenderLayer::LAYER0,
        );

        // We've got a simple UI attached to the pedestal, just a list of
        // places we can display, and a scale slider. It'll face towards the
        // user at fixed intervals, and won't slide around. This means it's
        // easy to access, but not hard to touch.
        let ui_dir = self.calc_pedestal_ui_dir();
        let mut ui_pose = Pose::new(
            ui_dir * (self.terrain.clip_radius + 0.04),
            ui::look_dir(ui_dir + VEC3_UP),
        );
        sk.render_add_model(
            &self.compass_model,
            ui::matrix_ts(
                ui_dir * (self.terrain.clip_radius + 0.01) + VEC3_UP * 0.02,
                0.4,
            ),
            named_colors::WHITE,
            RenderLayer::LAYER0,
        );

        ui::ui_window_begin("TerrainOptions", &mut ui_pose, Vec2::new(30.0, 0.0) * 0.01);

        // Show location buttons
        let mut new_location_id: Option<usize> = None;

        let btn_size = Vec2::new(6.0, 3.0) * 0.01;
        if ui::radio("Kauai", self.location_id == Some(0), btn_size) {
            dbg!("Location 0");
            new_location_id = Some(0);
        }
        ui::ui_sameline();
        if ui::radio("Grand Canyon", self.location_id == Some(1), btn_size) {
            dbg!("Location 1");
            new_location_id = Some(1);
        }
        ui::ui_sameline();
        if ui::radio("Mt. Everest", self.location_id == Some(2), btn_size) {
            dbg!("Location 2");
            new_location_id = Some(2);
        }
        ui::ui_sameline();
        if ui::radio("Machu Picchu", self.location_id == Some(3), btn_size) {
            dbg!("Location 3");
            new_location_id = Some(3);
        }

        if new_location_id.is_some() && new_location_id != self.location_id {
            self.load_location(sk, new_location_id).unwrap();
        }

        let mut ui_scale = self.terrain_scale;
        if ui::h_slider(
            "Scale",
            &mut ui_scale,
            0.00003,
            0.00005,
            0.0,
            27.0 * 0.01,
            ui::UIConfirm::Pinch,
        ) {
            self.set_scale(ui_scale);
        }

        ui::ui_window_end();

        self.show_terrain(sk).unwrap();

        ui::ui_handle_end();

        Ok(())
    }

    fn set_scale(&mut self, new_scale: f32) {
        self.terrain.set_heightmap_dimensions(
            self.map_height_size * new_scale,
            self.map_height_center * new_scale,
        );
        self.terrain.set_colormap_dimensions(
            self.map_color_size.xz() * new_scale,
            self.map_color_center * new_scale,
        );

        let geo_translation = self.terrain.get_local_position() / self.terrain_scale;
        self.terrain.set_local_position(geo_translation * new_scale);

        self.terrain_scale = new_scale;
    }

    fn _main(mut self, sk: SkSingle) -> anyhow::Result<()> {
        let floor = Floor::new(
            &sk,
            Vec2::new(10.0, 10.0),
            VEC3_UP,
            VEC3_FORWARD,
            "floor.png".to_string(),
        )?;

        let mut terrain = terrain::Terrain::new(&sk, 64, 0.6, 2)?;
        terrain.clip_radius = 0.3;

        self.load_location(&sk, Some(0))
            .expect("Unable to load location");

        sk.run(
            |sk_draw| {
                floor.draw(sk_draw);

                self.show_terrain_widget(sk_draw).unwrap();
            },
            |_| {},
        );

        Ok(())
    }

    fn calc_pedestal_ui_dir(&mut self) -> Vec3 {
        let head: Pose = unsafe { Pose::from(*stereokit_sys::input_head()) };
        let dir = (head.position - self.terrain_pose.position)
            .xz()
            .normalize_or_zero()
            .x0y();

        const SNAP_ANGLE: f32 = 60.0;
        const STICKY_AMOUNT: f32 = 20.0;
        let angle = dir.xz().angle();

        if angle_dist(angle, self.ui_angle) > SNAP_ANGLE / 2.0 + STICKY_AMOUNT {
            self.ui_angle = angle / SNAP_ANGLE * SNAP_ANGLE + SNAP_ANGLE / 2.0;
        }

        angle_xz(self.ui_angle, 0.0)
    }
}

pub fn angle_xz(angle_deg: f32, y: f32) -> Vec3 {
    Vec3::new(
        angle_deg.to_radians().cos(),
        y,
        angle_deg.to_radians().sin(),
    )
}

pub fn angle_dist(a: f32, b: f32) -> f32 {
    let delta = (b - a + 180.0) % 360.0 - 180.0;
    (if delta < -180.0 { delta + 360.0 } else { delta }).abs()
}

pub struct Floor {
    plane: Vec2,
    up: Vec3,
    forward: Vec3,
    texture: String,
    floor_mesh: Mesh,
    floor_mat: Material,
}

impl Floor {
    pub fn new(
        sk: &SkSingle,
        plane: Vec2,
        up: Vec3,
        forward: Vec3,
        texture: String,
    ) -> SkResult<Floor> {
        let floor_mesh = sk.mesh_gen_plane(plane, up, forward, 0, true);

        let floor_mat = sk.material_copy(Material::DEFAULT);
        sk.material_set_texture(
            &floor_mat,
            "diffuse",
            sk.tex_create_file(&texture, true, 10)?,
        );
        sk.material_set_float(&floor_mat, "tex_scale", 8.0);

        Ok(Floor {
            plane,
            up,
            forward,
            texture,
            floor_mesh,
            floor_mat,
        })
    }

    pub fn draw(&self, sk_draw: &SkDraw) {
        sk_draw.render_add_mesh(
            &self.floor_mesh,
            &self.floor_mat,
            Mat4::from_translation(Vec3 {
                x: 0.0,
                y: -1.5,
                z: 0.0,
            }),
            named_colors::WHITE,
            RenderLayer::LAYER0,
        );
    }
}
