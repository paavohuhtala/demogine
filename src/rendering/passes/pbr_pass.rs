// Temporary placeholder render pass while I'm working on deferred rendering
// This is called "PBR pass" despite doing nothing PBR-related

use std::sync::Arc;

use wgpu::{
    DepthBiasState, Device, MultisampleState, PipelineCompilationOptions, RenderPassDescriptor,
    StencilState,
};

use crate::rendering::{
    config::RenderConfig,
    instancing::{self, DrawableBuffers},
    mesh_buffers::MeshBuffers,
    passes::render_pass_context::{RenderPassContext, RenderPassCreationContext},
    render_model::{MODEL_PRIMITIVE_STATE, RENDER_MODEL_VBL},
    shader_loader::{RenderPipelineId, ShaderDefinition},
    texture::DepthTexture,
    util::bind_group_builder::BindGroupBuilder,
};

pub struct PbrPass {
    config: &'static RenderConfig,
    pipeline_id: RenderPipelineId,
    camera_bind_group: wgpu::BindGroup,
    mesh_buffers: Arc<MeshBuffers>,
    drawable_buffers: Arc<DrawableBuffers>,
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
    pub fn new(context: &mut RenderPassCreationContext) -> Self {
        let device = &context.shared.device;
        let common = context.shared.common.clone();

        let (camera_bind_group_layout, camera_bind_group) =
            BindGroupBuilder::new("Camera", wgpu::ShaderStages::VERTEX)
                .uniform(
                    0,
                    "Camera uniform buffer",
                    context.camera_uniform_buffer.as_entire_binding(),
                )
                .build(device);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Default shader render pipeline layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    context
                        .shared
                        .drawable_buffers
                        .visible_drawables
                        .bind_group_layout(),
                    context.material_manager.bind_group_layout(),
                ],
                push_constant_ranges: &[],
            });

        let pipeline_id = context.cache_builder.add_shader(
            DEFAULT_SHADER,
            Box::new(move |device: &Device, shader_module| {
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Default shader render pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: Some("vs_main"),
                        buffers: &[RENDER_MODEL_VBL],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
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
            }),
        );

        PbrPass {
            config: context.shared.config,
            pipeline_id,

            camera_bind_group,
            mesh_buffers: context.shared.mesh_buffers.clone(),
            drawable_buffers: context.shared.drawable_buffers.clone(),
        }
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
        render_pass.set_bind_group(1, self.drawable_buffers.visible_drawables.bind_group(), &[]);
        render_pass.set_bind_group(2, context.material_manager.bind_group(), &[]);

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
