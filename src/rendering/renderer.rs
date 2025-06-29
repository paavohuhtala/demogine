use std::sync::Arc;

use wgpu::CommandEncoderDescriptor;
use winit::window::Window;

use crate::{
    asset_pipeline::mesh_baker::BakedMeshes,
    demo::DemoState,
    math::frustum::Frustum,
    rendering::{
        common::Resolution,
        deferred::{
            gbuffer::GBuffer,
            geometry_pass::{GeometryPass, GeometryPassTextureViews},
        },
        global_uniform::GlobalUniformState,
        imgui_renderer::{create_imgui_renderer, ImguiRendererState},
        instancing::InstanceManager,
        mesh_buffers::MeshBuffers,
        passes::{
            background_pass::{BackgroundPass, BackgroundPassTextureViews},
            pbr_pass::{PbrPass, PbrTextureViews},
            render_pass_context::RenderPassContext,
        },
        render_camera::RenderCamera,
        render_common::RenderCommon,
        render_material_manager::RenderMaterialManager,
        shader_loader::{
            ComputeShaderLoader, PipelineCacheBuilder, RenderShaderLoader, ShaderLoader,
        },
        texture::DepthTexture,
    },
};

// TODO: this is a huge mess
pub struct Renderer {
    pub window: Arc<Window>,
    pub size: Resolution,
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    g_buffer: GBuffer,
    pub common: Arc<RenderCommon>,
    depth_texture: DepthTexture,
    camera: RenderCamera,
    imgui: ImguiRendererState,
    mesh_buffers: MeshBuffers,
    pub material_manager: RenderMaterialManager,

    render_shader_loader: RenderShaderLoader,
    background_pass: BackgroundPass,
    pbr_pass: PbrPass,
    geometry_pass: GeometryPass,

    compute_shader_loader: ComputeShaderLoader,
    instance_manager: InstanceManager,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        demo_state: &DemoState,
        baked_primitives: &BakedMeshes,
        imgui_context: &mut imgui::Context,
    ) -> anyhow::Result<Renderer> {
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
                required_features: wgpu::Features::MULTI_DRAW_INDIRECT
                    | wgpu::Features::INDIRECT_FIRST_INSTANCE
                    | wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                required_limits: wgpu::Limits {
                    max_binding_array_elements_per_shader_stage: 128,
                    ..Default::default()
                },
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

        let material_manager = RenderMaterialManager::new(&device, &queue);

        let mut render_pipeline_cache_builder = PipelineCacheBuilder::new();
        let background_pass =
            BackgroundPass::create(&device, common.clone(), &mut render_pipeline_cache_builder)?;
        let pbr_pass = PbrPass::create(
            &device,
            common.clone(),
            &mut render_pipeline_cache_builder,
            &material_manager,
        )?;
        let geometry_pass =
            GeometryPass::create(&device, common.clone(), &mut render_pipeline_cache_builder)?;
        let render_shader_loader = ShaderLoader::new(device.clone(), render_pipeline_cache_builder);

        let g_buffer = GBuffer::new(&device, size);

        let mesh_buffers = MeshBuffers::new(&device, baked_primitives);

        let mut compute_pipeline_cache_builder = PipelineCacheBuilder::new();
        let instance_manager = InstanceManager::new(
            &device,
            &mesh_buffers.meshes,
            &mut compute_pipeline_cache_builder,
        );
        let compute_shader_loader =
            ShaderLoader::new(device.clone(), compute_pipeline_cache_builder);

        let imgui = create_imgui_renderer(
            &device,
            &queue,
            common.output_surface_config.read().unwrap().format,
            imgui_context,
        );

        Ok(Self {
            window: window.clone(),
            surface,
            device,
            queue,
            g_buffer,
            common,
            size,
            camera,
            depth_texture,
            imgui,
            mesh_buffers,
            material_manager,

            render_shader_loader,
            background_pass,
            pbr_pass,
            geometry_pass,

            compute_shader_loader,
            instance_manager,
        })
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
        imgui_ui: &mut imgui::Ui,
    ) -> Result<RenderResult, wgpu::SurfaceError> {
        self.render_shader_loader
            .load_pending_shaders()
            .expect("Failed to load pending shaders");
        self.compute_shader_loader
            .load_pending_shaders()
            .expect("Failed to load pending compute shaders");

        self.camera.update_camera(&demo_state.camera);
        self.camera.update_uniform_buffer(&self.queue);
        self.common.global_uniform.update(
            &self.queue,
            GlobalUniformState::new(self.size, demo_state.start_time.elapsed().as_secs_f32()),
        );

        self.instance_manager
            .update_from_scene(&demo_state.scene, &self.queue, imgui_ui);

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let frustum = Frustum::from_view_projection(*self.camera.get_view_proj());
        self.instance_manager.cull_and_generate_commands(
            &self.queue,
            &mut encoder,
            &self.compute_shader_loader.cache,
            &frustum,
        );

        let pipeline_cache = &self.render_shader_loader.cache;

        self.background_pass.render(
            &BackgroundPassTextureViews {
                color: view.clone(),
            },
            &mut encoder,
            pipeline_cache,
        );

        let indirect_buffer = self.instance_manager.draw_commands_buffer();

        let unified_bind_group = self.instance_manager.bind_group();

        let mut pass_context = RenderPassContext {
            encoder: &mut encoder,
            pipeline_cache,
            instance_bind_group: &unified_bind_group,
            indirect_buffer,
            mesh_buffers: &self.mesh_buffers,
            material_manager: &mut self.material_manager,
        };

        self.pbr_pass.render_indirect(
            &PbrTextureViews {
                color: view.clone(),
                depth: self.depth_texture.view().clone(),
            },
            &mut pass_context,
        );

        self.geometry_pass.render_indirect(
            &GeometryPassTextureViews {
                color_roughness: self.g_buffer.color_roughness.view.clone(),
                normal_metallic: self.g_buffer.normal_metallic.view.clone(),
                depth: self.g_buffer.depth.view().clone(),
            },
            &mut pass_context,
        );

        Ok(RenderResult {
            output,
            view,
            encoder,
        })
    }

    // Rendering is split to render() and finish_frame() to allow drawing to imgui during rendering
    // Otherwise &mut imgui::Ui and &mut imgui::Context would have a conflict
    pub fn finish_frame(
        &mut self,
        RenderResult {
            mut encoder,
            output,
            view,
        }: RenderResult,
        imgui_context: &mut imgui::Context,
    ) {
        self.imgui.render(
            &view,
            imgui_context,
            &self.device,
            &self.queue,
            &mut encoder,
        );

        let command_buffer = encoder.finish();
        self.queue.submit([command_buffer]);

        self.window.pre_present_notify();
        output.present();
    }
}

pub struct RenderResult {
    output: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}
