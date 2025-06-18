use glam::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;

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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CameraUniform {
    view_proj: Mat4,
}

impl CameraUniform {
    pub fn update(&mut self, resolution: winit::dpi::PhysicalSize<u32>, camera: &Camera) {
        self.view_proj =
            camera.get_vp_matrix(Vec2::new(resolution.width as f32, resolution.height as f32));
    }

    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn update_buffer(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[*self]));
    }
}
