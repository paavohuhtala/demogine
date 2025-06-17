use std::sync::Arc;

use anyhow::Context;
use glam::{Mat4, Quat, Vec2, Vec3};
use itertools::Itertools;
use rand::Rng;
use wgpu::{
    util::DeviceExt, CommandEncoderDescriptor, DepthBiasState, Device, MultisampleState,
    PipelineCompilationOptions, RenderPassDescriptor, ShaderSource, StencilState,
};
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::EventLoop, window::Window,
};

use crate::{
    camera::{Camera, CameraUniform},
    model::{Instance, Model, RenderModel, RENDER_MODEL_VBL},
    shader_loader::{PipelineCacheBuilder, PipelineId, ShaderDefinition, ShaderLoader},
    texture::DepthTexture,
};

struct GraphicsState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,

    depth_texture: DepthTexture,

    default_pipeline_id: PipelineId,
    fullscreen_quad: RenderModel,
    render_model: RenderModel,

    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    instance_buffer: wgpu::Buffer,
    shader_loader: ShaderLoader,
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

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let mut camera_uniform = CameraUniform::default();
        camera_uniform.update(size, &game_state.camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let depth_texture = DepthTexture::new(&device, &config, "Depth Texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        const DEFAULT_SHADER: ShaderDefinition = ShaderDefinition {
            name: "Default Shader",
            path: "shader.wgsl",
        };

        let mut cache_builder: PipelineCacheBuilder = PipelineCacheBuilder::new();
        let default_pipeline_id = cache_builder.add_shader(
            DEFAULT_SHADER,
            Box::new(
                move |device: &Device, shader_def: &ShaderDefinition, source: &str| {
                    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(shader_def.name),
                        source: ShaderSource::Wgsl(source.into()),
                    });

                    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("Render pipeline"),
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: Some("vs_main"),
                            buffers: &[RENDER_MODEL_VBL, Instance::descriptor()],
                            compilation_options: PipelineCompilationOptions::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: Some("fs_main"),
                            targets: &[Some(wgpu::ColorTargetState {
                                format: config.format,
                                blend: Some(wgpu::BlendState::REPLACE),
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
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

        let shader_loader = ShaderLoader::new(device.clone(), cache_builder);

        let render_model = {
            let (document, buffers, _images) = gltf::import("assets/spacefarjan.glb")?;
            let ship: gltf::Mesh<'_> = document.meshes().next().context("No meshes in gltf")?;
            let model = Model::from_gtlf(ship, &buffers).context("Failed to create model")?;
            RenderModel::from_model(&device, model)
        };

        let fullscreen_quad = {
            let quad_model = Model::quad();
            RenderModel::from_model(&device, quad_model)
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
            config,
            size,
            default_pipeline_id,
            fullscreen_quad,
            render_model,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            depth_texture,
            instance_buffer,
            shader_loader,
        })
    }

    fn resize(&mut self, game_state: &GameState, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.depth_texture.resize(&self.device, &self.config);

            self.camera_uniform.update(new_size, &game_state.camera);
            self.queue.write_buffer(
                &self.camera_buffer,
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
        mouse_pos: Vec2,
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

        let relative_mouse_pos =
            mouse_pos / Vec2::new(self.size.width as f32, self.size.height as f32);

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: relative_mouse_pos.x as f64,
                            g: 0.5,
                            b: relative_mouse_pos.y as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let pipeline = self.shader_loader.cache.get(self.default_pipeline_id);
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

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
        }

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
