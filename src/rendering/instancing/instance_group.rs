use std::collections::BTreeMap;

use id_arena::Id;

use crate::rendering::{
    instancing::{
        instance_data::InstanceData, instance_storage_buffer::InstanceStorageBuffer, InstanceType,
    },
    render_model::RenderModel,
};

#[derive(Debug, Clone)]
pub struct InstanceBatch {
    pub instance_type: InstanceType,
    pub render_model_id: Id<RenderModel>,
    pub start_index: u32,
    pub instance_count: u32,
}

#[derive(Debug)]
struct ModelGroup(Vec<InstanceData>);

impl ModelGroup {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn count(&self) -> usize {
        self.0.len()
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn add_instance(&mut self, instance: InstanceData) {
        self.0.push(instance);
    }
}

pub struct InstanceGroup {
    instance_type: InstanceType,
    model_groups: BTreeMap<Id<RenderModel>, ModelGroup>,
    batches: Vec<InstanceBatch>,
    storage_buffer: InstanceStorageBuffer,
    pub needs_rebuild: bool,
}

impl InstanceGroup {
    pub fn new(device: &wgpu::Device, instance_type: InstanceType, initial_capacity: u64) -> Self {
        Self {
            instance_type,
            model_groups: BTreeMap::new(),
            batches: Vec::new(),
            storage_buffer: InstanceStorageBuffer::new(device, initial_capacity),
            needs_rebuild: true,
        }
    }

    pub fn clear(&mut self) {
        self.model_groups.clear();
    }

    pub fn clear_model_groups(&mut self) {
        for group in self.model_groups.values_mut() {
            group.clear();
        }
    }

    pub fn retain_non_empty_groups(&mut self) {
        self.model_groups.retain(|_, group| !group.is_empty());
    }

    pub fn rebuild_and_upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.batches.clear();

        let mut current_index = 0u32;
        let mut total_instances = 0u32;
        for (&render_model_id, group) in self.model_groups.iter() {
            if !group.0.is_empty() {
                self.batches.push(InstanceBatch {
                    instance_type: self.instance_type,
                    render_model_id,
                    start_index: current_index,
                    instance_count: group.count() as u32,
                });

                current_index += group.count() as u32;
                total_instances += group.count() as u32;
            }
        }

        self.storage_buffer
            .ensure_capacity(device, total_instances as u64);

        for batch in &self.batches {
            if let Some(group) = self.model_groups.get(&batch.render_model_id) {
                self.storage_buffer
                    .write_instances_at_offset(queue, &group.0, batch.start_index);
            }
        }

        self.needs_rebuild = false;
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        self.storage_buffer.bind_group()
    }

    pub fn batches(&self) -> &[InstanceBatch] {
        &self.batches
    }

    pub fn add_instance(&mut self, model_id: Id<RenderModel>, instance: InstanceData) {
        self.model_groups
            .entry(model_id)
            .or_insert_with(ModelGroup::new)
            .add_instance(instance);
    }

    pub fn instance_count(&self) -> usize {
        self.model_groups.values().map(|group| group.count()).sum()
    }

    pub fn model_count(&self) -> usize {
        self.model_groups.len()
    }

    pub fn debug_view(&self, imgui_ui: &imgui::Ui) {
        let instances = self.instance_count();
        let capacity = self.storage_buffer.capacity();

        imgui_ui.text(format!(
            "  Capacity: {} instances",
            self.storage_buffer.capacity()
        ));
        let dynamic_size_mb =
            (capacity as f64 * std::mem::size_of::<InstanceData>() as f64) / (1024.0 * 1024.0);
        imgui_ui.text(format!("  Size: {:.2} MB", dynamic_size_mb));
        let dynamic_usage = if capacity > 0 {
            (instances as f64 / capacity as f64) * 100.0
        } else {
            0.0
        };
        imgui_ui.text(format!("  Usage: {:.1}%", dynamic_usage));
    }
}
