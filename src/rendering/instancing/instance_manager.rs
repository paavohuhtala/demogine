use crate::{
    rendering::instancing::{
        instance_data::InstanceData,
        instance_group::{InstanceBatch, InstanceGroup},
        InstanceType,
    },
    scene_graph::scene::Scene,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum UpdateStrategy {
    /// Only dynamic instances changed (most common case)
    DynamicOnly,
    /// Static instances changed, need to update everything (rare but expensive)
    UpdateAll,
}

pub struct InstanceManager {
    static_group: InstanceGroup,
    dynamic_group: InstanceGroup,
}

impl InstanceManager {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            static_group: InstanceGroup::new(device, InstanceType::Static, 1024),
            dynamic_group: InstanceGroup::new(device, InstanceType::Dynamic, 16),
        }
    }

    /// Main entry point: gather instances and update GPU data
    pub fn update_from_scene(
        &mut self,
        scene: &Scene,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        imgui_ui: &mut imgui::Ui,
    ) {
        // Phase 1: Gather instances from scene
        self.gather_instances_from_scene(scene);

        // Phase 2: Determine what needs updating
        let update_strategy = self.determine_update_strategy();

        // Phase 3: Update GPU data based on strategy
        self.update_gpu_data(update_strategy, device, queue);

        // ImGui debug window
        self.render_debug_window(imgui_ui);
    }

    /// Collect instances from the scene, separating static and dynamic
    /// Static objects are only rebuilt when explicitly requested
    fn gather_instances_from_scene(&mut self, scene: &Scene) {
        self.process_dynamic_objects(scene);

        if self.static_group.needs_rebuild {
            self.rebuild_static_objects(scene);
        }

        // Remove empty groups
        self.static_group.retain_non_empty_groups();
        self.dynamic_group.retain_non_empty_groups();
    }

    fn process_objects(&mut self, scene: &Scene, instance_type: InstanceType) {
        let instance_group = match instance_type {
            InstanceType::Static => &mut self.static_group,
            InstanceType::Dynamic => &mut self.dynamic_group,
        };

        for (_, object) in scene.objects.iter() {
            if !object.enabled {
                continue;
            }

            if object.instance_type != instance_type {
                continue;
            }

            // Skip objects without models
            let Some(model_id) = object.model_id else {
                continue;
            };
            let Some(model) = scene.models.get(model_id) else {
                continue;
            };
            let Some(render_model_id) = model.render_model else {
                continue;
            };

            let transform_matrix = *object.transform.get_world_matrix();
            let instance = InstanceData::new(transform_matrix);
            instance_group.add_instance(render_model_id, instance);
        }
    }

    /// Process dynamic objects - called every frame
    fn process_dynamic_objects(&mut self, scene: &Scene) {
        self.dynamic_group.clear_model_groups();
        self.process_objects(scene, InstanceType::Dynamic);
    }

    /// Rebuild all static objects - only called when static_needs_rebuild is true
    fn rebuild_static_objects(&mut self, scene: &Scene) {
        self.static_group.clear();
        self.process_objects(scene, InstanceType::Static);
    }

    /// Determine what kind of update is needed
    fn determine_update_strategy(&mut self) -> UpdateStrategy {
        if self.static_group.needs_rebuild {
            self.dynamic_group.needs_rebuild = true;
            UpdateStrategy::UpdateAll
        } else {
            self.dynamic_group.needs_rebuild = true;
            UpdateStrategy::DynamicOnly
        }
    }

    /// Update GPU data based on the determined strategy
    fn update_gpu_data(
        &mut self,
        strategy: UpdateStrategy,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        match strategy {
            UpdateStrategy::DynamicOnly => {
                // Only rebuild dynamic data
                self.dynamic_group.rebuild_and_upload(device, queue);
            }
            UpdateStrategy::UpdateAll => {
                // Rebuild both static and dynamic data
                self.static_group.rebuild_and_upload(device, queue);
                self.dynamic_group.rebuild_and_upload(device, queue);
            }
        }
    }

    /// Get the static bind group for rendering
    pub fn static_bind_group(&self) -> &wgpu::BindGroup {
        self.static_group.bind_group()
    }

    /// Get the dynamic bind group for rendering
    pub fn dynamic_bind_group(&self) -> &wgpu::BindGroup {
        self.dynamic_group.bind_group()
    }

    /// Get all batches (both static and dynamic) for rendering
    pub fn all_batches(&self) -> impl Iterator<Item = &InstanceBatch> {
        self.static_group
            .batches()
            .iter()
            .chain(self.dynamic_group.batches().iter())
    }

    /// Force rebuild of static instances (e.g., when scene changes)
    #[allow(dead_code)]
    pub fn request_static_rebuild(&mut self) {
        self.static_group.needs_rebuild = true;
    }

    /// Render ImGui debug window with detailed statistics
    fn render_debug_window(&self, imgui_ui: &imgui::Ui) {
        imgui_ui
            .window("Instance Manager Debug")
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .size([500.0, 600.0], imgui::Condition::FirstUseEver)
            .build(|| {
                // General statistics
                imgui_ui.text("=== General Stats ===");
                imgui_ui.text(format!(
                    "Static Models: {}",
                    self.static_group.model_count()
                ));
                imgui_ui.text(format!(
                    "Dynamic Models: {}",
                    self.dynamic_group.model_count()
                ));

                let static_instances = self.static_group.instance_count();
                let dynamic_instances = self.dynamic_group.instance_count();
                imgui_ui.text(format!("Static Instances: {}", static_instances));
                imgui_ui.text(format!("Dynamic Instances: {}", dynamic_instances));
                imgui_ui.text(format!(
                    "Total Instances: {}",
                    static_instances + dynamic_instances
                ));
                imgui_ui.separator();

                // GPU buffer statistics
                imgui_ui.text("=== GPU Buffers ===");

                imgui_ui.text("Static Buffer:");
                self.static_group.debug_view(imgui_ui);
                imgui_ui.text("Dynamic Buffer:");
                self.dynamic_group.debug_view(imgui_ui);
            });
    }
}
