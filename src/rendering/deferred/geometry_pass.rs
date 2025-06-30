use std::sync::Arc;

use wgpu::{
    LoadOp, PipelineCompilationOptions, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineDescriptor, StoreOp,
    TextureView, VertexState,
};

use crate::rendering::{
    config::RenderConfig,
    deferred::gbuffer::GBuffer,
    instancing::{self, DrawableBuffers},
    mesh_buffers::MeshBuffers,
    passes::render_pass_context::{RenderPassContext, RenderPassCreationContext},
    render_model::RENDER_MODEL_VBL,
    shader_loader::{RenderPipelineId, ShaderDefinition},
    texture::DepthTexture,
    util::bind_group_builder::BindGroupBuilder,
};

pub struct GeometryPass {
    config: &'static RenderConfig,
    pipeline_id: RenderPipelineId,
    camera_bind_group: wgpu::BindGroup,
    mesh_buffers: Arc<MeshBuffers>,
    drawable_buffers: Arc<DrawableBuffers>,
}

pub struct GeometryPassTextureViews {
    pub color_roughness: TextureView,
    pub normal_metallic: TextureView,
    pub depth: TextureView,
}

const SHADER_DEF: ShaderDefinition = ShaderDefinition {
    name: "Geometry pass shader",
    path: "deferred/geometry.wgsl",
};

impl GeometryPass {
    pub fn new(context: &mut RenderPassCreationContext) -> Self {
        let device = &context.shared.device;
        let common = context.shared.common.clone();
        let config = context.shared.config;
        let cache_builder = &mut context.cache_builder;

        let (camera_bind_group_layout, camera_bind_group) =
            BindGroupBuilder::new("camera", wgpu::ShaderStages::VERTEX)
                .uniform(
                    0,
                    "Camera uniform buffer",
                    common.camera_uniform_buffer.as_entire_binding(),
                )
                .build(device);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render pipeline layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    context
                        .shared
                        .drawable_buffers
                        .visible_drawables
                        .bind_group_layout(),
                ],
                push_constant_ranges: &[],
            });

        let pipeline_id = cache_builder.add_shader(
            SHADER_DEF,
            Box::new(move |device, shader_module| {
                let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("Geometry pass render pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: VertexState {
                        module: &shader_module,
                        entry_point: Some("vs_main"),
                        buffers: &[RENDER_MODEL_VBL],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: Some("fs_main"),
                        targets: &[
                            Some(wgpu::ColorTargetState {
                                format: GBuffer::COLOR_ROUGHNESS_FORMAT,
                                blend: Some(wgpu::BlendState::REPLACE),
                                write_mask: wgpu::ColorWrites::ALL,
                            }),
                            Some(wgpu::ColorTargetState {
                                format: GBuffer::NORMAL_METALLIC_FORMAT,
                                blend: Some(wgpu::BlendState::REPLACE),
                                write_mask: wgpu::ColorWrites::ALL,
                            }),
                        ],
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
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

                Ok(pipeline)
            }),
        );

        GeometryPass {
            config,
            pipeline_id,
            camera_bind_group,
            mesh_buffers: context.shared.mesh_buffers.clone(),
            drawable_buffers: context.shared.drawable_buffers.clone(),
        }
    }

    pub fn render_indirect(
        &self,
        texture_views: &GeometryPassTextureViews,
        context: &mut RenderPassContext,
    ) {
        let mut render_pass = context.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Geometry pass (Indirect)"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: &texture_views.color_roughness,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: &texture_views.normal_metallic,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &texture_views.depth,
                depth_ops: Some(wgpu::Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let pipeline = context.pipeline_cache.get(self.pipeline_id);
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, self.drawable_buffers.visible_drawables.bind_group(), &[]);

        render_pass.set_vertex_buffer(0, self.mesh_buffers.vertices.slice(..));
        render_pass.set_index_buffer(
            self.mesh_buffers.indices.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        if self.config.use_multi_draw_indirect_count {
            render_pass.multi_draw_indexed_indirect_count(
                context.draw_commands_buffer,
                0,
                context.draw_commands_count_buffer,
                0,
                instancing::MAX_MESHES as u32,
            );
        } else {
            render_pass.multi_draw_indexed_indirect(
                context.draw_commands_buffer,
                0,
                instancing::MAX_MESHES as u32,
            );
        }
    }
}
