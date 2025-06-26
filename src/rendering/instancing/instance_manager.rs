use crate::{
    math::frustum::Frustum,
    rendering::{
        frustum_culling::FrustumCullingResources,
        instancing::{drawable::Drawable, drawable_storage_buffer::DrawableStorageBuffer},
    },
    scene_graph::scene::Scene,
};

pub struct InstanceManager {
    drawable_buffer: DrawableStorageBuffer,
    drawables: Vec<Drawable>,
    frustum_culling: FrustumCullingResources,
}

impl InstanceManager {
    pub fn new(device: &wgpu::Device, primitive_buffer: &wgpu::Buffer) -> Self {
        let drawable_buffer = DrawableStorageBuffer::new(device, 64_000);

        let frustum_culling =
            FrustumCullingResources::new(device, drawable_buffer.buffer(), primitive_buffer);

        Self {
            drawable_buffer,
            frustum_culling,
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
                self.drawables
                    .push(Drawable::new(matrix, primitive.global_index as u32));
            }
        }

        // Pre-sort by primitive index to enable draw command merging
        self.drawables.sort_by_key(|d| d.primitive_index);

        imgui_ui
            .window("Instance Manager")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                imgui_ui.text(format!("Total drawables: {}", self.drawables.len()));
            });
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        self.drawable_buffer.bind_group()
    }

    pub fn cull_and_generate_commands(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frustum: &Frustum,
    ) {
        self.frustum_culling.update_frustum(queue, frustum);
        self.frustum_culling
            .dispatch_culling_for_buffer(encoder, self.drawables.len() as u32);
    }

    pub fn draw_commands_buffer(&self) -> &wgpu::Buffer {
        &self.frustum_culling.draw_commands_buffer
    }
}
