// Temporary placeholder render pass while I'm working on deferred rendering
// This is called "PBR pass" despite doing nothing PBR-related

use wgpu::{
    DepthBiasState, Device, MultisampleState, PipelineCompilationOptions, RenderPassDescriptor,
    ShaderSource, StencilState,
};

use crate::rendering::{
    passes::render_pass_context::RenderPassContext,
    render_common::RenderCommon,
    render_material_manager::RenderMaterialManager,
    render_model::{MODEL_PRIMITIVE_STATE, RENDER_MODEL_VBL},
    shader_loader::{self, RenderPipelineId, ShaderDefinition},
    texture::DepthTexture,
};

pub struct PbrPass {
    pub pipeline_id: RenderPipelineId,
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

impl PbrPass {
    pub fn create(
        device: &wgpu::Device,
        common: std::sync::Arc<RenderCommon>,
        cache_builder: &mut shader_loader::PipelineCacheBuilder<wgpu::RenderPipeline>,
        material_manager: &RenderMaterialManager,
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

        let instance_storage_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Drawable bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Default shader render pipeline layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &instance_storage_group_layout,
                    material_manager.bind_group_layout(),
                ],
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
                            buffers: &[RENDER_MODEL_VBL],
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
                        primitive: MODEL_PRIMITIVE_STATE,
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

    pub fn render_indirect(
        &self,
        texture_views: &PbrTextureViews,
        context: &mut RenderPassContext,
    ) {
        let mut render_pass = context.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("PBR Pass (Indirect)"),
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

        let pipeline = context.pipeline_cache.get(self.pipeline_id);
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, context.instance_bind_group, &[]);
        render_pass.set_bind_group(2, context.material_manager.bind_group(), &[]);

        render_pass.set_vertex_buffer(0, context.mesh_buffers.vertices.slice(..));
        render_pass.set_index_buffer(
            context.mesh_buffers.indices.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.multi_draw_indexed_indirect(context.indirect_buffer, 0, 32_000);
    }
}
