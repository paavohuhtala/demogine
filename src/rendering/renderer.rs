use std::sync::Arc;

use glam::Vec2;
use id_arena::Arena;
use wgpu::CommandEncoderDescriptor;
use winit::window::Window;

use crate::{
    demo::DemoState,
    rendering::{
        common::Resolution,
        deferred::gbuffer::GBuffer,
        global_uniform::GlobalUniformState,
        instance::gather_instances,
        passes::{
            background_pass::{BackgroundPass, BackgroundPassTextureViews},
            pbr_pass::{PbrPass, PbrTextureViews},
        },
        render_camera::RenderCamera,
        render_common::RenderCommon,
        render_model::{render_model_instances, RenderModel},
        shader_loader::{PipelineCacheBuilder, ShaderLoader},
        texture::DepthTexture,
    },
};

pub struct Renderer {
    pub window: Arc<Window>,
    pub size: Resolution,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,

    g_buffer: GBuffer,

    common: Arc<RenderCommon>,
    depth_texture: DepthTexture,
    render_models: Arena<RenderModel>,

    camera: RenderCamera,

    shader_loader: ShaderLoader,

    background_pass: BackgroundPass,
    pbr_pass: PbrPass,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, demo_state: &DemoState) -> anyhow::Result<Renderer> {
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

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
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

    pub fn render(
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

        gather_instances(&demo_state.scene, &mut self.render_models);

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
        );

        self.pbr_pass.render(
            &PbrTextureViews {
                color: view.clone(),
                depth: self.depth_texture.view().clone(),
            },
            &mut encoder,
            pipeline_cache,
            |render_pass| {
                for (_id, render_model) in self.render_models.iter() {
                    if !render_model.instances.should_render() {
                        continue;
                    }

                    render_model_instances(render_pass, &self.queue, render_model);
                }
            },
        );

        let command_buffer = encoder.finish();

        self.queue.submit([command_buffer]);

        output.present();

        Ok(())
    }
}
