use std::time::Instant;

use anyhow::Context;
use glam::{Quat, Vec3};

use crate::{
    camera::Camera,
    rendering::instancing::InstanceType,
    scene_graph::{object3d::ObjectId, scene::Scene},
};

pub struct DemoState {
    pub camera: Camera,
    pub start_time: Instant,
    pub scene: Scene,
    can: ObjectId,
    extra_cans: Vec<ObjectId>,
    last_cans_randomization: Instant,
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

        for x in -25..25 {
            for z in -25..25 {
                let translation = Vec3::new(x as f32 * 0.5, 0.0, z as f32 * 0.5);
                // Look towards 0.0, 0.0, 0.0
                let rotation =
                    Quat::from_axis_angle(Vec3::Y, (x as f32 * 0.1).atan2(z as f32 * 0.1));
                let scale = 0.5;

                let can = scene
                    .spawn_gltf_scene(&buffers, &can_scene, InstanceType::Static)
                    .expect("Expected scene to contain a root node");

                scene.set_object_transform(can, translation, rotation, scale);
            }
        }

        let can = scene
            .spawn_gltf_scene(&buffers, &can_scene, InstanceType::Dynamic)
            .expect("Expected scene to contain a root node");

        let extra_cans = (0..1000)
            .map(|_| {
                let can = scene
                    .spawn_gltf_scene(&buffers, &can_scene, InstanceType::Dynamic)
                    .expect("Expected scene to contain a root node");

                scene.set_object_scale(can, 0.1);

                can
            })
            .collect();

        Ok(Self {
            camera,
            start_time: Instant::now(),
            scene,
            can,
            extra_cans,
            last_cans_randomization: Instant::now(),
        })
    }

    pub fn update(&mut self) {
        let time = self.start_time.elapsed().as_secs_f32();

        let rotation = Quat::from_axis_angle(Vec3::Y, time * 0.5);

        let translation = Vec3::Y * (time * 2.0).sin() * 0.05;

        self.scene
            .set_object_transform(self.can, translation, rotation, 1.0);

        // Rotate camera around the origin
        let camera_rotation = Quat::from_axis_angle(Vec3::Y, time * 0.1);
        self.camera.eye = camera_rotation * Vec3::new(1.0, 2.0, 1.0);

        randomize_cans(self, Instant::now());
    }
}

fn randomize_cans(state: &mut DemoState, now: Instant) {
    if now
        .duration_since(state.last_cans_randomization)
        .as_secs_f32()
        > 1.0
    {
        state.last_cans_randomization = now;

        // Randomize the position of the cans
        for can in state.extra_cans.iter_mut() {
            let translation = Vec3::new(
                rand::random::<f32>() * 10.0 - 5.0,
                1.5,
                rand::random::<f32>() * 10.0 - 5.0,
            );

            state.scene.set_object_translation(*can, translation);
        }
    }
}
