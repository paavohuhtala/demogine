pub struct Texture {
    _texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_wgpu_texture(texture: wgpu::Texture, device: &wgpu::Device) -> Self {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            _texture: texture,
            view,
            _sampler: sampler,
        }
    }
}

pub struct DepthTexture {
    texture: Texture,
    label: String,
}

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: impl Into<String>,
    ) -> Self {
        let label: String = label.into();
        let texture = Self::create_wgpu_texture(device, config, &label);

        DepthTexture {
            texture: Texture::from_wgpu_texture(texture, device),
            label,
        }
    }

    fn create_wgpu_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> wgpu::Texture {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let descriptor = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let wgpu_texture = device.create_texture(&descriptor);

        wgpu_texture
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.texture = Texture::from_wgpu_texture(
            Self::create_wgpu_texture(device, config, &self.label),
            device,
        );
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.texture.view
    }
}
