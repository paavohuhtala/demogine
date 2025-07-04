use wgpu::{MultisampleState, PipelineCompilationOptions, RenderPassDescriptor};

use crate::rendering::{
    passes::render_pass_context::RenderPassCreationContext,
    shader_loader::{RenderPipelineCache, RenderPipelineId, ShaderDefinition},
};

pub struct BackgroundPass {
    pipeline_id: RenderPipelineId,
    global_uniform_bind_group: wgpu::BindGroup,
}

const FULLSCREEN_QUAD_SHADER: ShaderDefinition = ShaderDefinition {
    name: "Fullscreen Quad",
    path: "fullscreen_quad.wgsl",
};

pub struct BackgroundPassTextureViews {
    pub color: wgpu::TextureView,
}

impl BackgroundPass {
    pub fn create(context: &mut RenderPassCreationContext) -> anyhow::Result<BackgroundPass> {
        let device = &context.shared.device;
        let common = context.shared.common.clone();

        let global_uniform_bind_group = common.global_uniform.bind_group.clone();

        let quad_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Quad Render Pipeline Layout"),
                bind_group_layouts: &[&common.global_uniform.bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline_id = context.cache_builder.add_shader(
            FULLSCREEN_QUAD_SHADER,
            Box::new(move |device, shader_module| {
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Background Pass Pipeline"),
                    layout: Some(&quad_render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: Some("vs_main"),
                        buffers: &[],
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
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

                Ok(pipeline)
            }),
        );

        Ok(Self {
            pipeline_id,
            global_uniform_bind_group,
        })
    }

    pub fn render(
        &self,
        texture_views: &BackgroundPassTextureViews,
        encoder: &mut wgpu::CommandEncoder,
        pipeline_cache: &RenderPipelineCache,
    ) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Background Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_views.color,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let pipeline = pipeline_cache.get(self.pipeline_id);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &self.global_uniform_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
