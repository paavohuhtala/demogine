use wgpu::{
    DepthBiasState, Device, MultisampleState, PipelineCompilationOptions, RenderPass,
    RenderPassDescriptor, ShaderSource, StencilState,
};

use crate::{
    model::{Instance, RENDER_MODEL_VBL},
    passes::pass::Pass,
    shader_loader::{self, PipelineCache, PipelineId, ShaderDefinition},
    texture::DepthTexture,
};

pub struct PbrPass {
    pub pipeline_id: PipelineId,
    camera_bind_group: wgpu::BindGroup,
}

pub struct PbrTextureViews {
    pub color: wgpu::TextureView,
    pub depth: wgpu::TextureView,
}

const DEFAULT_SHADER: ShaderDefinition = ShaderDefinition {
    name: "Default Shader",
    path: "shader.wgsl",
};

impl Pass for PbrPass {
    type TextureViews = PbrTextureViews;

    fn create(
        device: &wgpu::Device,
        common: std::sync::Arc<crate::render_common::RenderCommon>,
        cache_builder: &mut shader_loader::PipelineCacheBuilder,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: common.camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline_id = cache_builder.add_shader(
            DEFAULT_SHADER,
            Box::new(
                move |device: &Device, shader_def: &ShaderDefinition, source: &str| {
                    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(shader_def.name),
                        source: ShaderSource::Wgsl(source.into()),
                    });

                    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("Default shader render pipeline"),
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: Some("vs_main"),
                            buffers: &[RENDER_MODEL_VBL, Instance::descriptor()],
                            compilation_options: PipelineCompilationOptions::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: Some("fs_main"),
                            targets: &[Some(wgpu::ColorTargetState {
                                format: common.output_surface_config.read().unwrap().format,
                                blend: Some(wgpu::BlendState::REPLACE),
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                            compilation_options: PipelineCompilationOptions::default(),
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            strip_index_format: None,
                            front_face: wgpu::FrontFace::Cw,
                            cull_mode: Some(wgpu::Face::Back),
                            polygon_mode: wgpu::PolygonMode::Fill,
                            unclipped_depth: false,
                            conservative: false,
                        },
                        depth_stencil: Some(wgpu::DepthStencilState {
                            format: DepthTexture::DEPTH_FORMAT,
                            depth_write_enabled: true,
                            depth_compare: wgpu::CompareFunction::Less,
                            stencil: StencilState::default(),
                            bias: DepthBiasState::default(),
                        }),
                        multisample: MultisampleState::default(),
                        multiview: None,
                        cache: None,
                    });

                    Ok(pipeline)
                },
            ),
        );

        Ok(PbrPass {
            pipeline_id,
            camera_bind_group,
        })
    }

    fn render<'a, F>(
        &self,
        texture_views: &Self::TextureViews,
        encoder: &mut wgpu::CommandEncoder,
        pipeline_cache: &PipelineCache,
        render_callback: F,
    ) where
        F: FnOnce(&mut RenderPass) + 'a,
    {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("PBR Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_views.color,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &texture_views.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let pipeline = pipeline_cache.get(self.pipeline_id);
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        render_callback(&mut render_pass);
    }
}
