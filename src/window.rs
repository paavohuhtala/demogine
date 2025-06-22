use std::sync::Arc;

use anyhow::Context;
use glam::Vec2;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::EventLoop, window::Window,
};

use crate::{demo::DemoState, rendering::renderer::Renderer};

struct App {
    renderer: Option<Renderer>,
    demo_state: DemoState,
    mouse_pos: Vec2,
}

impl App {
    fn from_demo_state(demo_state: DemoState) -> Self {
        Self {
            renderer: None,
            demo_state,
            mouse_pos: Vec2::ZERO,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = event_loop.create_window(window_attributes).unwrap();
        let state = pollster::block_on(Renderer::new(Arc::new(window), &self.demo_state)).unwrap();
        self.renderer = Some(state);

        self.renderer
            .as_mut()
            .unwrap()
            .load_models(&mut self.demo_state)
            .unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.renderer.as_mut().unwrap().resize(new_size);
            }
            WindowEvent::RedrawRequested => {
                let state = self.renderer.as_mut().unwrap();
                state.window.request_redraw();

                self.demo_state.update();
                // TODO: This needs a better place
                self.demo_state.scene.update_transforms();

                match state.render(&mut self.demo_state, self.mouse_pos) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size);
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
    }
}

pub async fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    let demo_state = DemoState::new().context("Failed to create game state")?;
    let mut app = App::from_demo_state(demo_state);
    event_loop.run_app(&mut app)?;

    Ok(())
}
