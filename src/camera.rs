use std::cell::{Cell, Ref, RefCell};

use glam::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

#[derive(Debug, Clone)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
}

impl Camera {
    pub fn get_vp_matrix(&self, resolution: Vec2) -> Mat4 {
        let view = Mat4::look_at_lh(self.eye, self.target, self.up);
        let projection = Mat4::perspective_lh(45.0, resolution.x / resolution.y, 0.1, 100.0);
        projection * view
    }
}

pub struct RenderCamera {
    camera: Camera,
    resolution: PhysicalSize<u32>,
    view_proj: RefCell<Mat4>,
    is_dirty: Cell<bool>,
    should_update_uniform: Cell<bool>,
    pub uniform_buffer: wgpu::Buffer,
}

impl RenderCamera {
    pub fn new(device: &wgpu::Device, camera: Camera, resolution: PhysicalSize<u32>) -> Self {
        let matrix =
            camera.get_vp_matrix(Vec2::new(resolution.width as f32, resolution.height as f32));

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[matrix]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            camera,
            resolution,
            view_proj: RefCell::new(matrix),
            is_dirty: Cell::new(true),
            should_update_uniform: Cell::new(true),
            uniform_buffer,
        }
    }

    fn invalidate(&self) {
        self.is_dirty.set(true);
        self.should_update_uniform.set(true);
    }

    pub fn update_resolution(&mut self, resolution: PhysicalSize<u32>) {
        if self.resolution != resolution {
            self.resolution = resolution;
            self.invalidate();
        }
    }

    pub fn update_camera(&mut self, camera: &Camera) {
        self.camera = camera.clone();
        self.invalidate();
    }

    fn update_view_proj(&self) {
        if !self.is_dirty.get() {
            return;
        }

        *self.view_proj.borrow_mut() = self.camera.get_vp_matrix(Vec2::new(
            self.resolution.width as f32,
            self.resolution.height as f32,
        ));
        self.is_dirty.set(false);
    }

    pub fn get_view_proj(&self) -> Ref<Mat4> {
        self.update_view_proj();
        self.view_proj.borrow()
    }

    pub fn update_uniform_buffer(&self, queue: &wgpu::Queue) {
        if !self.should_update_uniform.get() {
            return;
        }

        let view_proj = *self.get_view_proj();

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[view_proj]));

        self.should_update_uniform.set(false);
    }
}
