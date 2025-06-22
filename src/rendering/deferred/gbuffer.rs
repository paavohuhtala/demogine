use wgpu::TextureFormat;

use crate::rendering::{
    common::{PhysicalSizeExt, Resolution},
    texture::{DepthTexture, Texture},
};

pub struct GBuffer {
    device: wgpu::Device,

    /// In 8-bit sRGB. RGB for base color, A for roughness.
    pub color_rougness: Texture,
    /// In 16-bit float. RGB for normal, A for metallic.
    pub normal_metallic: Texture,
    // 32-bit float depth texture
    pub depth: DepthTexture,
}

impl GBuffer {
    pub const COLOR_ROUGHNESS_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
    pub const NORMAL_METALLIC_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

    pub fn new(device: &wgpu::Device, size: Resolution) -> Self {
        let color_roughness = Self::create_color_roughness_texture(device, size);
        let normal_metallic = Self::create_normal_metallic_texture(device, size);
        let depth = Self::create_depth_texture(device, size);

        Self {
            device: device.clone(),
            color_rougness: color_roughness,
            normal_metallic,
            depth,
        }
    }

    fn create_attachment(
        device: &wgpu::Device,
        label: &'static str,
        format: TextureFormat,
        size: Resolution,
    ) -> Texture {
        let descriptor = wgpu::TextureDescriptor {
            label: Some(label),
            size: size.to_extent3d(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&descriptor);
        Texture::from_wgpu_texture(device, descriptor, texture, None)
    }

    fn create_color_roughness_texture(device: &wgpu::Device, size: Resolution) -> Texture {
        Self::create_attachment(
            device,
            "GBuffer base color and roughness",
            Self::COLOR_ROUGHNESS_FORMAT,
            size,
        )
    }

    fn create_normal_metallic_texture(device: &wgpu::Device, size: Resolution) -> Texture {
        Self::create_attachment(
            device,
            "GBuffer normal and metallic",
            Self::NORMAL_METALLIC_FORMAT,
            size,
        )
    }
    pub fn create_depth_texture(device: &wgpu::Device, size: Resolution) -> DepthTexture {
        DepthTexture::new(device, size, "GBuffer depth texture")
    }

    pub fn resize(&mut self, size: Resolution) {
        self.color_rougness.resize(&self.device, size);
        self.normal_metallic.resize(&self.device, size);
        self.depth.resize(&self.device, size);
    }
}
