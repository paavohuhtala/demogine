use anyhow::Context;
use glam::{Quat, Vec3};

use crate::{camera::Camera, scene_graph::object3d::ObjectId, scene_graph::scene::Scene};

pub struct DemoState {
    pub camera: Camera,
    pub start_time: std::time::Instant,
    pub scene: Scene,
    can: ObjectId,
}

impl DemoState {
    pub fn new() -> anyhow::Result<Self> {
        let camera = Camera {
            eye: Vec3::new(1.0, 2.0, 1.0),
            target: Vec3::new(0.0, 1.0, 0.0),
            up: Vec3::Y,
        };

        let mut scene = Scene::new();

        let (document, buffers, _images) = gltf::import("assets/can/can.gltf")?;
        let can_scene = document.scenes().next().context("No scenes in gltf")?;
        scene.spawn_gltf_scene(&buffers, &can_scene);

        let can = scene
            .get_object_by_name("Tolkki")
            .expect("Can object not found");

        Ok(Self {
            camera,
            start_time: std::time::Instant::now(),
            scene,
            can,
        })
    }

    pub fn update(&mut self) {
        let time = self.start_time.elapsed().as_secs_f32();

        let rotation = Quat::from_axis_angle(Vec3::Y, time * 0.5);

        let translation = Vec3::Y * (time * 2.0).sin() * 0.05;

        self.scene
            .set_object_transform(self.can, translation, rotation, 1.0);
    }
}
