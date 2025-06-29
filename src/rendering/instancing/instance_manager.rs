use crate::{
    math::frustum::Frustum,
    rendering::{
        config::RenderConfig,
        instancing::{
            draw_command_generator::DrawCommandGenerator, drawable::Drawable,
            drawable_storage_buffer::DrawableStorageBuffer,
        },
        shader_loader::{ComputePipelineCache, PipelineCacheBuilder},
    },
    scene_graph::scene::Scene,
};

pub struct InstanceManager {
    drawable_buffer: DrawableStorageBuffer,
    visible_drawable_buffer: DrawableStorageBuffer,
    drawables: Vec<Drawable>,
    draw_command_generator: DrawCommandGenerator,
}

impl InstanceManager {
    pub fn new(
        device: &wgpu::Device,
        config: &'static RenderConfig,
        mesh_info_buffer: &wgpu::Buffer,
        pipeline_builder: &mut PipelineCacheBuilder<wgpu::ComputePipeline>,
    ) -> Self {
        let drawable_buffer = DrawableStorageBuffer::new(device, 32_000);
        let visible_drawable_buffer = DrawableStorageBuffer::new(device, 32_000);

        let draw_command_generator = DrawCommandGenerator::new(
            device,
            config,
            drawable_buffer.buffer(),
            visible_drawable_buffer.buffer(),
            mesh_info_buffer,
            pipeline_builder,
        );

        Self {
            drawable_buffer,
            visible_drawable_buffer,
            draw_command_generator,
            drawables: Vec::new(),
        }
    }

    pub fn update_from_scene(&mut self, scene: &Scene, queue: &wgpu::Queue, imgui_ui: &imgui::Ui) {
        self.gather_drawables_from_scene(scene, imgui_ui);

        self.drawable_buffer
            .write_drawables_at_offset(queue, &self.drawables, 0);
    }

    fn gather_drawables_from_scene(&mut self, scene: &Scene, imgui_ui: &imgui::Ui) {
        self.drawables.clear();

        for (_, object) in scene.objects.iter() {
            if !object.enabled {
                continue;
            }

            let Some(model_id) = object.model_id else {
                continue;
            };

            let Some(model) = scene.models.get(model_id) else {
                continue;
            };

            let matrix = object.transform.get_world_matrix().clone();

            for primitive in &model.model.primitives {
                self.drawables.push(Drawable::new(
                    matrix,
                    primitive.global_index as u32,
                    primitive.material_id.index() as u32,
                ));
            }
        }

        imgui_ui
            .window("Instance Manager")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                imgui_ui.text(format!("Total drawables: {}", self.drawables.len()));
            });
    }

    pub fn visible_drawable_bind_group(&self) -> &wgpu::BindGroup {
        self.visible_drawable_buffer.bind_group()
    }

    pub fn cull_and_generate_commands(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        pipeline_cache: &ComputePipelineCache,
        frustum: &Frustum,
    ) {
        self.draw_command_generator.update_frustum(queue, frustum);
        self.draw_command_generator
            .dispatch(encoder, pipeline_cache, self.drawables.len() as u32);
    }

    pub fn draw_commands_buffer(&self) -> &wgpu::Buffer {
        &self.draw_command_generator.draw_commands_buffer
    }

    pub fn draw_commands_count_buffer(&self) -> &wgpu::Buffer {
        self.draw_command_generator.draw_commands_count_buffer()
    }
}
