use std::sync::Arc;

use anyhow::Context;
use glam::Vec2;
use id_arena::Arena;
use wgpu::CommandEncoderDescriptor;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::EventLoop, window::Window,
};

use crate::{
    camera::RenderCamera,
    demo::DemoState,
    global_uniform::GlobalUniformState,
    model::RenderModel,
    passes::{
        background_pass::{BackgroundPass, BackgroundPassTextureViews},
        pass::Pass,
        pbr_pass::{PbrPass, PbrTextureViews},
    },
    render_common::RenderCommon,
    rendering::deferred::gbuffer::GBuffer,
    shader_loader::{PipelineCacheBuilder, ShaderLoader},
    texture::DepthTexture,
};

struct GraphicsState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,

    g_buffer: GBuffer,

    common: Arc<RenderCommon>,
    depth_texture: DepthTexture,
    render_models: Arena<RenderModel>,

    camera: RenderCamera,

    shader_loader: ShaderLoader,

    background_pass: BackgroundPass,
    pbr_pass: PbrPass,
}

impl GraphicsState {
    async fn new(window: Arc<Window>, demo_state: &DemoState) -> anyhow::Result<GraphicsState> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let camera = RenderCamera::new(&device, demo_state.camera.clone(), size);

        let common = RenderCommon::new(
            &device,
            &adapter,
            &surface,
            size,
            camera.uniform_buffer.clone(),
        );
        let common = Arc::new(common);

        let depth_texture = DepthTexture::new(&device, size, "Depth Texture");

        let mut cache_builder: PipelineCacheBuilder = PipelineCacheBuilder::new();

        let background_pass = BackgroundPass::create(&device, common.clone(), &mut cache_builder)?;
        let pbr_pass = PbrPass::create(&device, common.clone(), &mut cache_builder)?;

        let shader_loader = ShaderLoader::new(device.clone(), cache_builder);

        let g_buffer = GBuffer::new(&device, size);

        let render_models = Arena::new();

        Ok(Self {
            window: window.clone(),
            surface,
            device,
            queue,
            g_buffer,
            common,
            size,
            render_models,
            camera,
            depth_texture,
            shader_loader,

            background_pass,
            pbr_pass,
        })
    }

    pub fn load_models(&mut self, demo_state: &mut DemoState) -> anyhow::Result<()> {
        for (_id, scene_model) in &mut demo_state.scene.models {
            let render_model = RenderModel::from_model(&self.device, &scene_model.model);
            let render_model_id = self.render_models.alloc(render_model);
            scene_model.render_model = Some(render_model_id);
            println!(
                "Loaded model {} with {} primitives",
                scene_model.name,
                scene_model.model.primitives.len()
            );
        }

        Ok(())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        let common = self.common.as_ref();
        let mut config = common.output_surface_config.write().unwrap();

        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            config.width = new_size.width;
            config.height = new_size.height;
            self.depth_texture.resize(&self.device, new_size);
            self.surface.configure(&self.device, &config);
            self.camera.update_resolution(new_size);
            self.g_buffer.resize(new_size);
        }
    }

    fn render(
        &mut self,
        demo_state: &mut DemoState,
        _mouse_pos: Vec2,
    ) -> Result<(), wgpu::SurfaceError> {
        self.shader_loader
            .load_pending_shaders()
            .expect("Failed to load pending shaders");

        self.camera.update_camera(&demo_state.camera);
        self.camera.update_uniform_buffer(&self.queue);
        self.common.global_uniform.update(
            &self.queue,
            GlobalUniformState::new(self.size, demo_state.start_time.elapsed().as_secs_f32()),
        );

        // TODO: Update transform matrices
        demo_state.scene.gather_instances();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let pipeline_cache = &self.shader_loader.cache;

        self.background_pass.render(
            &BackgroundPassTextureViews {
                color: view.clone(),
            },
            &mut encoder,
            pipeline_cache,
            |render_pass| {
                render_pass.draw(0..3, 0..1);
            },
        );

        if true {
            self.pbr_pass.render(
                &PbrTextureViews {
                    color: view.clone(),
                    depth: self.depth_texture.view().clone(),
                },
                &mut encoder,
                pipeline_cache,
                |render_pass| {
                    for (_id, scene_model) in &demo_state.scene.models {
                        if !scene_model.instances().should_render() {
                            continue;
                        }

                        let render_model_id = scene_model
                            .render_model
                            .with_context(|| {
                                format!("Scene model has no render model: {}", scene_model.name)
                            })
                            .unwrap();

                        let render_model = self
                            .render_models
                            .get(render_model_id)
                            .with_context(|| {
                                format!(
                                    "Render model not found in graphics state: {}",
                                    scene_model.name
                                )
                            })
                            .unwrap();

                        // Update GPU-side instance buffer
                        // This could probably be done earlier in the frame, or even in a separate thread
                        scene_model
                            .instances()
                            .write_to_buffer(&self.queue, &render_model.instance_buffer);

                        render_model.instance_buffer.bind(render_pass);

                        for primitive in render_model.primitives.iter() {
                            render_pass.set_vertex_buffer(0, primitive.vertex_buffer.slice(..));
                            render_pass.set_index_buffer(
                                primitive.index_buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );

                            render_pass.draw_indexed(
                                0..primitive.num_indices,
                                0,
                                0..scene_model.instances().len() as u32,
                            );
                        }
                    }
                },
            );
        }

        let command_buffer = encoder.finish();

        self.queue.submit([command_buffer]);

        output.present();

        Ok(())
    }
}

struct App {
    graphics_state: Option<GraphicsState>,
    demo_state: DemoState,
    mouse_pos: Vec2,
}

impl App {
    fn from_demo_state(demo_state: DemoState) -> Self {
        Self {
            graphics_state: None,
            demo_state,
            mouse_pos: Vec2::ZERO,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = event_loop.create_window(window_attributes).unwrap();
        let state =
            pollster::block_on(GraphicsState::new(Arc::new(window), &self.demo_state)).unwrap();
        self.graphics_state = Some(state);

        self.graphics_state
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
                self.graphics_state.as_mut().unwrap().resize(new_size);
            }
            WindowEvent::RedrawRequested => {
                let state = self.graphics_state.as_mut().unwrap();
                state.window.request_redraw();

                self.demo_state.update();

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
