use std::{sync::Arc, time::Instant};

use anyhow::Context;
use glam::Vec2;
use imgui::{FontConfig, FontSource};
use imgui_winit_support::WinitPlatform;
use winit::{
    application::ApplicationHandler,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::{
    asset_pipeline::mesh_baker::{bake_models, BakedMeshes},
    demo::DemoState,
    engine,
    rendering::renderer::Renderer,
};

struct ImguiState {
    context: imgui::Context,
    platform: WinitPlatform,
}

struct App {
    renderer: Option<Renderer>,
    demo_state: DemoState,
    mouse_pos: Vec2,
    imgui: Option<ImguiState>,
    last_frame: Instant,
    frame_time_ms: f32,
    baked_primitives: BakedMeshes,
}

impl App {
    fn from_demo_state(demo_state: DemoState) -> Self {
        // This doesn't really belong here
        let models = demo_state
            .scene
            .models
            .iter()
            .map(|(_, model)| &model.model)
            .collect::<Vec<_>>();
        let baked_primitives = bake_models(&models);

        Self {
            renderer: None,
            demo_state,
            mouse_pos: Vec2::ZERO,
            imgui: None,
            last_frame: Instant::now(),
            frame_time_ms: 0.0,
            baked_primitives,
        }
    }

    fn setup_imgui(&mut self, window: &Window) {
        let mut context = imgui::Context::create();
        let mut platform = WinitPlatform::new(&mut context);
        platform.attach_window(
            context.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );

        let font_size = 14.0;
        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        // Disable INI support because it's broken in the published version of imgui
        context.set_ini_filename(None);

        self.imgui = Some(ImguiState { context, platform });
    }

    fn show_frame_time_overlay(ui: &imgui::Ui, frame_time_ms: f32) {
        let window_size = ui.io().display_size;
        let overlay_pos = [window_size[0] - 120.0, 10.0];

        ui.window("Frame Time")
            .position(overlay_pos, imgui::Condition::Always)
            .size([110.0, 60.0], imgui::Condition::Always)
            .no_decoration()
            .no_inputs()
            .bg_alpha(0.8)
            .build(|| {
                ui.text(format!("Frame: {:.2} ms", frame_time_ms));
                ui.text(format!("FPS: {:.0}", 1000.0 / frame_time_ms.max(0.001)));
            });
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = event_loop.create_window(window_attributes).unwrap();
        self.setup_imgui(&window);
        let state = pollster::block_on(Renderer::new(
            Arc::new(window),
            &self.demo_state,
            &self.baked_primitives,
            &mut self.imgui.as_mut().unwrap().context,
        ))
        .unwrap();
        self.renderer = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let imgui = self.imgui.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.renderer.as_mut().unwrap().resize(new_size);
            }
            WindowEvent::RedrawRequested => {
                let delta_time = self.last_frame.elapsed();
                let now = Instant::now();
                self.frame_time_ms = delta_time.as_secs_f32() * 1000.0;
                imgui.context.io_mut().update_delta_time(delta_time);
                self.last_frame = now;

                let renderer = self.renderer.as_mut().unwrap();
                renderer.window.request_redraw();

                imgui
                    .platform
                    .prepare_frame(imgui.context.io_mut(), &renderer.window)
                    .expect("Failed to prepare Imgui frame");
                let ui = imgui.context.new_frame();

                let frame_time_ms = self.frame_time_ms;
                Self::show_frame_time_overlay(&ui, frame_time_ms);

                engine::update(&mut self.demo_state, renderer, ui)
                    .expect("Error during engine::update");

                match renderer.render(&mut self.demo_state, ui) {
                    Ok(result) => {
                        renderer.finish_frame(result, &mut imgui.context);
                    }
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        renderer.resize(renderer.size);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("Out of memory");
                        event_loop.exit();
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        log::warn!("Timeout");
                    }
                    Err(other) => {
                        log::error!("Unexpected error: {:?}", other);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
            }
            _ => (),
        }

        {
            let window = self.renderer.as_mut().unwrap().window.as_ref();
            imgui.platform.handle_event::<()>(
                imgui.context.io_mut(),
                &window,
                &Event::WindowEvent { window_id, event },
            );
        }
    }
}

pub async fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    let demo_state = DemoState::new().context("Failed to create game state")?;
    let mut app = App::from_demo_state(demo_state);
    event_loop.run_app(&mut app)?;

    Ok(())
}
