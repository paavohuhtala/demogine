use std::num::NonZeroU32;

use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureDescriptor};

use crate::{asset_pipeline::materials::PbrMaterialData, material_manager::MaterialManager};

pub struct TextureEntry {
    #[allow(dead_code)]
    pub ty: TextureType,
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PbrMaterialInfo {
    pub base_color: u32,
    pub normal: u32,
    pub ao_roughness_metallic: u32,
    _padding: u32,
}

pub struct RenderMaterialManager {
    device: wgpu::Device,
    queue: wgpu::Queue,

    textures: Vec<TextureEntry>,
    materials: Vec<PbrMaterialInfo>,

    material_info_buffer: Option<wgpu::Buffer>,
    sampler: wgpu::Sampler,

    bind_group_layout: wgpu::BindGroupLayout,
    // Created lazily
    bind_group: Option<wgpu::BindGroup>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureType {
    BaseColor,
    Normal,
    AoRoughnessMetallic,
}

impl RenderMaterialManager {
    // This could be handled dynamically, but we don't need that for a demo
    pub const MAX_TEXTURE_COUNT: u32 = 128;

    const DEFAULT_TEXTURE_BASE_COLOR: usize = 0;
    const DEFAULT_TEXTURE_NORMAL: usize = 1;
    const DEFAULT_TEXTURE_AO_ROUGHNESS_METALLIC: usize = 2;

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let default_base_color =
            Self::create_default_texture(device, queue, TextureType::BaseColor);
        let default_normal = Self::create_default_texture(device, queue, TextureType::Normal);
        let default_ao_roughness_metallic =
            Self::create_default_texture(device, queue, TextureType::AoRoughnessMetallic);

        let textures = vec![
            default_base_color,
            default_normal,
            default_ao_roughness_metallic,
        ];

        let materials = Vec::new();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Default sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture manager bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: Some(NonZeroU32::new(Self::MAX_TEXTURE_COUNT).unwrap()),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        Self {
            device: device.clone(),
            queue: queue.clone(),

            textures,
            materials,

            material_info_buffer: None,
            sampler,

            bind_group_layout,
            bind_group: None,
        }
    }

    pub fn load_material(&mut self, pbr_material: &PbrMaterialData) -> usize {
        let base_color = if let Some(data) = &pbr_material.base_color {
            self.create_texture(&pbr_material.name, TextureType::BaseColor, data)
        } else {
            Self::DEFAULT_TEXTURE_BASE_COLOR
        };

        let normal = if let Some(data) = &pbr_material.normal {
            self.create_texture(&pbr_material.name, TextureType::Normal, data)
        } else {
            Self::DEFAULT_TEXTURE_NORMAL
        };

        let ao_roughness_metallic = if let Some(data) = &pbr_material.ao_roughness_metallic {
            self.create_texture(&pbr_material.name, TextureType::AoRoughnessMetallic, data)
        } else {
            Self::DEFAULT_TEXTURE_AO_ROUGHNESS_METALLIC
        };

        let material_info = PbrMaterialInfo {
            base_color: base_color as u32,
            normal: normal as u32,
            ao_roughness_metallic: ao_roughness_metallic as u32,
            _padding: 0,
        };

        let material_index = self.materials.len();
        self.materials.push(material_info);
        material_index
    }

    pub fn load_all_materials(&mut self, material_manager: &MaterialManager) {
        for pbr_material in material_manager.materials() {
            self.load_material(pbr_material);
        }

        let material_info_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Material info buffer"),
                    contents: bytemuck::cast_slice(&self.materials),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        self.material_info_buffer = Some(material_info_buffer);
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    // This is &mut self for now, but we can avoid that later with a Cell/RefCell if needed
    pub fn bind_group(&mut self) -> &wgpu::BindGroup {
        if self.bind_group.is_some() {
            return self.bind_group.as_ref().unwrap();
        }

        let mut texture_views = Vec::with_capacity(Self::MAX_TEXTURE_COUNT as usize);

        for texture in &self.textures {
            texture_views.push(&texture.view);
        }

        // Fill the rest with copies of the default texture view
        let default_texture_view = self.textures[0].view.clone();
        while texture_views.len() < Self::MAX_TEXTURE_COUNT as usize {
            texture_views.push(&default_texture_view);
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture manager bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self
                        .material_info_buffer
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureViewArray(&texture_views),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.bind_group = Some(bind_group);
        self.bind_group.as_ref().unwrap()
    }

    fn create_texture(
        &mut self,
        name: &str,
        texture_type: TextureType,
        texture_data: &gltf::image::Data,
    ) -> usize {
        let label = format!("{name}({:?})", texture_type);
        get_texture_format_from_type(texture_type);

        let texture = self.device.create_texture_with_data(
            &self.queue,
            &TextureDescriptor {
                label: Some(&label),
                size: wgpu::Extent3d {
                    width: texture_data.width,
                    height: texture_data.height,
                    depth_or_array_layers: 1,
                },
                // TODO: Generate mipmaps
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: get_texture_format_from_type(texture_type),
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::wgt::TextureDataOrder::default(),
            &texture_data.pixels,
        );

        // TODO: Default view is probably not what we want
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let texture_entry = TextureEntry {
            ty: texture_type,
            texture,
            view,
        };

        let texture_index = self.textures.len();
        self.textures.push(texture_entry);
        texture_index
    }

    fn create_default_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_type: TextureType,
    ) -> TextureEntry {
        let label = format!("Default Texture ({:?})", texture_type);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&label),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: get_texture_format_from_type(texture_type),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let placeholder_data = match texture_type {
            TextureType::BaseColor => [255, 0, 255, 255],
            TextureType::Normal => [127, 127, 255, 255],
            TextureType::AoRoughnessMetallic => [0, 255, 0, 255],
        };

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&[placeholder_data]),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        TextureEntry {
            ty: texture_type,
            texture,
            view,
        }
    }
}

fn get_texture_format_from_type(texture_type: TextureType) -> wgpu::TextureFormat {
    match texture_type {
        TextureType::BaseColor => wgpu::TextureFormat::Rgba8UnormSrgb,
        TextureType::Normal => wgpu::TextureFormat::Rgba8Unorm,
        TextureType::AoRoughnessMetallic => wgpu::TextureFormat::Rgba8Unorm,
    }
}
