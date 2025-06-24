use imgui_wgpu::RendererConfig;
use wgpu::{CommandEncoder, TextureView};

pub struct ImguiRendererState {
    renderer: imgui_wgpu::Renderer,
}

impl ImguiRendererState {
    pub fn render(
        &mut self,
        view: &TextureView,
        context: &mut imgui::Context,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Imgui render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let draw_data = context.render();

        // Workaround for memory safety related crash in imgui-rs
        // https://github.com/imgui-rs/imgui-rs/issues/325
        if draw_data.draw_lists_count() == 0 {
            return;
        }

        self.renderer
            .render(draw_data, queue, device, &mut render_pass)
            .expect("Rendering Imgui failed");
    }
}

pub fn create_imgui_renderer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_format: wgpu::TextureFormat,
    context: &mut imgui::Context,
) -> ImguiRendererState {
    let renderer_config = RendererConfig {
        texture_format,
        ..Default::default()
    };

    let imgui_renderer = imgui_wgpu::Renderer::new(context, device, queue, renderer_config);

    ImguiRendererState {
        renderer: imgui_renderer,
    }
}
