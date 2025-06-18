use std::sync::RwLock;

use wgpu::{Buffer, SurfaceConfiguration};
use winit::dpi::PhysicalSize;

use crate::camera::CameraUniform;

pub struct RenderCommon {
    pub output_surface_config: RwLock<SurfaceConfiguration>,
    pub camera_buffer: Buffer,
}

impl RenderCommon {
    pub fn new(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        surface: &wgpu::Surface,
        size: PhysicalSize<u32>,
    ) -> Self {
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let output_surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(device, &output_surface_config);

        let default_camera_uniform = CameraUniform::default();
        let camera_buffer = default_camera_uniform.create_buffer(device);

        Self {
            output_surface_config: RwLock::new(output_surface_config),
            camera_buffer,
        }
    }
}
