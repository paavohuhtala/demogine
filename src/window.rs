use std::{ops::Deref, sync::Arc};

use anyhow::Context;
use glam::{Mat4, Quat, Vec2, Vec3};
use itertools::Itertools;
use rand::Rng;
use wgpu::CommandEncoderDescriptor;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::EventLoop, window::Window,
};

use crate::{
    camera::{Camera, CameraUniform},
    model::{Instance, Model, RenderModel},
    passes::{
        background_pass::{BackgroundPass, BackgroundPassTextureViews},
        pass::Pass,
        pbr_pass::{PbrPass, PbrTextureViews},
    },
    render_common::RenderCommon,
    shader_loader::{PipelineCacheBuilder, ShaderLoader},
    texture::DepthTexture,
};

struct GraphicsState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,

    common: Arc<RenderCommon>,
    depth_texture: DepthTexture,
    render_model: RenderModel,

    camera_uniform: CameraUniform,

    instance_buffer: wgpu::Buffer,
    shader_loader: ShaderLoader,

    background_pass: BackgroundPass,
    pbr_pass: PbrPass,
}

impl GraphicsState {
    async fn new(window: Arc<Window>, game_state: &GameState) -> anyhow::Result<GraphicsState> {
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

        let common = RenderCommon::new(&device, &adapter, &surface, size);
        let common = Arc::new(common);

        let mut camera_uniform = CameraUniform::default();
        camera_uniform.update(size, &game_state.camera);

        let depth_texture = {
            let surface_config = common.output_surface_config.read().unwrap();
            let surface_config = surface_config.deref();
            DepthTexture::new(&device, surface_config, "Depth Texture")
        };

        let mut cache_builder: PipelineCacheBuilder = PipelineCacheBuilder::new();

        let background_pass = BackgroundPass::create(&device, common.clone(), &mut cache_builder)?;
        let pbr_pass = PbrPass::create(&device, common.clone(), &mut cache_builder)?;

        let shader_loader = ShaderLoader::new(device.clone(), cache_builder);

        let render_model = {
            let (document, buffers, _images) = gltf::import("assets/spacefarjan.glb")?;
            let ship: gltf::Mesh<'_> = document.meshes().next().context("No meshes in gltf")?;
            let model = Model::from_gtlf(ship, &buffers).context("Failed to create model")?;
            RenderModel::from_model(&device, model)
        };

        let instance_buffer_descriptor = wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (std::mem::size_of::<Instance>() * game_state.instances.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        };

        let instance_buffer = device.create_buffer(&instance_buffer_descriptor);

        queue.write_buffer(
            &instance_buffer,
            0,
            bytemuck::cast_slice(&game_state.instances),
        );

        Ok(Self {
            window: window.clone(),
            surface,
            device,
            queue,
            common,
            size,
            render_model,
            camera_uniform,
            depth_texture,
            instance_buffer,
            shader_loader,

            background_pass,
            pbr_pass,
        })
    }

    fn resize(&mut self, game_state: &GameState, new_size: winit::dpi::PhysicalSize<u32>) {
        let common = self.common.as_ref();
        let mut config = common.output_surface_config.write().unwrap();

        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            config.width = new_size.width;
            config.height = new_size.height;
            self.surface.configure(&self.device, &config);

            self.depth_texture.resize(&self.device, &config);

            self.camera_uniform.update(new_size, &game_state.camera);
            self.queue.write_buffer(
                &common.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform]),
            );
        }
    }

    fn update(&mut self, game_state: &GameState) {
        self.camera_uniform.update(self.size, &game_state.camera);
    }

    fn render(
        &mut self,
        game_state: &GameState,
        _mouse_pos: Vec2,
    ) -> Result<(), wgpu::SurfaceError> {
        self.shader_loader
            .load_pending_shaders()
            .expect("Failed to load pending shaders");

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

        self.pbr_pass.render(
            &PbrTextureViews {
                color: view.clone(),
                depth: self.depth_texture.view().clone(),
            },
            &mut encoder,
            pipeline_cache,
            |render_pass| {
                render_pass.set_vertex_buffer(0, self.render_model.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

                render_pass.set_index_buffer(
                    self.render_model.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(
                    0..self.render_model.num_indices as u32,
                    0,
                    0..game_state.instances.len() as u32,
                );
            },
        );

        let command_buffer = encoder.finish();

        self.queue.submit([command_buffer]);

        output.present();

        Ok(())
    }
}

struct GameState {
    instances: Vec<Instance>,
    camera: Camera,
}

impl GameState {
    fn new() -> Self {
        let camera = Camera {
            eye: Vec3::new(10.0, 10.0, 10.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
        };

        let rng = rand::thread_rng();
        let instances = (0..10)
            .flat_map(|i| {
                let mut rng = rng.clone();
                (0..10).map(move |j| {
                    let position = Vec3::new(i as f32 * 2.0, 0.0, j as f32 * 2.0);
                    let rotation = Vec3::new(
                        rng.gen_range(0.0..std::f32::consts::PI),
                        rng.gen_range(0.0..std::f32::consts::PI),
                        rng.gen_range(0.0..std::f32::consts::PI),
                    );

                    let model = Mat4::from_rotation_translation(
                        Quat::from_euler(glam::EulerRot::XYZ, rotation.x, rotation.y, rotation.z),
                        position,
                    );

                    Instance { model }
                })
            })
            .collect_vec();

        Self { instances, camera }
    }
}

struct App {
    graphics_state: Option<GraphicsState>,
    game_state: GameState,
    mouse_pos: Vec2,
}

impl App {
    fn from_game_state(game_state: GameState) -> Self {
        Self {
            graphics_state: None,
            game_state,
            mouse_pos: Vec2::ZERO,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = event_loop.create_window(window_attributes).unwrap();
        let state =
            pollster::block_on(GraphicsState::new(Arc::new(window), &self.game_state)).unwrap();
        self.graphics_state = Some(state);
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
                self.graphics_state
                    .as_mut()
                    .unwrap()
                    .resize(&self.game_state, new_size);
            }
            WindowEvent::RedrawRequested => {
                let state = self.graphics_state.as_mut().unwrap();
                state.window.request_redraw();
                state.update(&self.game_state);

                match state.render(&self.game_state, self.mouse_pos) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(&self.game_state, state.size);
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
    let game_state = GameState::new();
    let mut app = App::from_game_state(game_state);
    event_loop.run_app(&mut app)?;

    Ok(())
}
