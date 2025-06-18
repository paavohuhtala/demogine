use std::sync::Arc;

use wgpu::{
    Device, MultisampleState, PipelineCompilationOptions, RenderPass, RenderPassDescriptor,
    ShaderSource,
};

use crate::{
    passes::pass::Pass,
    render_common::RenderCommon,
    shader_loader::{PipelineCache, PipelineId, ShaderDefinition},
};

pub struct BackgroundPass {
    pipeline_id: PipelineId,
}

const FULLSCREEN_QUAD_SHADER: ShaderDefinition = ShaderDefinition {
    name: "Fullscreen Quad",
    path: "fullscreen_quad.wgsl",
};

pub struct BackgroundPassTextureViews {
    pub color: wgpu::TextureView,
}

impl Pass for BackgroundPass {
    type TextureViews = BackgroundPassTextureViews;

    fn create(
        device: &Device,
        common: Arc<RenderCommon>,
        cache_builder: &mut crate::shader_loader::PipelineCacheBuilder,
    ) -> anyhow::Result<BackgroundPass> {
        let quad_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Quad Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let fullscreen_quad_pipeline_id = cache_builder.add_shader(
            FULLSCREEN_QUAD_SHADER,
            Box::new(
                move |device: &Device, shader_def: &ShaderDefinition, source: &str| {
                    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(shader_def.name),
                        source: ShaderSource::Wgsl(source.into()),
                    });

                    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("Background Pass Pipeline"),
                        layout: Some(&quad_render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: Some("vs_main"),
                            buffers: &[],
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
                },
            ),
        );

        Ok(Self {
            pipeline_id: fullscreen_quad_pipeline_id,
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
        render_callback(&mut render_pass);
    }
}
