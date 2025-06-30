use std::sync::Arc;

use crate::{
    math::frustum::Frustum,
    rendering::{
        instancing::{
            draw_command_generator::DrawCommandGenerator, drawable::Drawable, DrawableBuffers,
        },
        passes::render_pass_context::ComputePassCreationContext,
        shader_loader::ComputePipelineCache,
    },
    scene_graph::scene::Scene,
};

pub struct DrawableManager {
    drawable_buffers: Arc<DrawableBuffers>,
    drawables: Vec<Drawable>,
    draw_command_generator: DrawCommandGenerator,
}

impl DrawableManager {
    pub fn new(context: &mut ComputePassCreationContext) -> Self {
        let draw_command_generator = DrawCommandGenerator::new(context);

        Self {
            drawable_buffers: context.shared.drawable_buffers.clone(),
            draw_command_generator,
            drawables: Vec::new(),
        }
    }

    pub fn update_from_scene(&mut self, scene: &Scene, queue: &wgpu::Queue, imgui_ui: &imgui::Ui) {
        self.gather_drawables_from_scene(scene, imgui_ui);

        self.drawable_buffers
            .all_drawables
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
            let inverse_transpose_matrix = object
                .transform
                .get_inverse_transpose_world_matrix()
                .clone();

            for primitive in &model.model.primitives {
                self.drawables.push(Drawable::new(
                    matrix,
                    inverse_transpose_matrix,
                    primitive.global_index as u32,
                    primitive.material_id.index() as u32,
                ));
            }

            /*self.drawables.iter_mut().for_each(|drawable| {
                drawable.calculate_inverse_transpose_model_matrix();
            });*/
        }

        imgui_ui
            .window("Instance Manager")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                imgui_ui.text(format!("Total drawables: {}", self.drawables.len()));
            });
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
