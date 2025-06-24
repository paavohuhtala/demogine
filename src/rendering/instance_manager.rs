use glam::Mat4;
use id_arena::Id;
use std::collections::HashMap;
use wgpu::BufferUsages;

use crate::{rendering::render_model::RenderModel, scene_graph::scene::Scene};

/// This should match the same structure defined in WGSL
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub model_matrix: Mat4,
}

impl InstanceData {
    pub fn new(model_matrix: Mat4) -> Self {
        Self { model_matrix }
    }
}

/// A batch of instances for a specific model
#[derive(Debug, Clone)]
pub struct InstanceBatch {
    pub render_model_id: Id<RenderModel>,
    pub start_index: u32,
    pub instance_count: u32,
}

/// Per-model instance collection with change tracking
#[derive(Debug)]
struct ModelInstanceGroup {
    instances: Vec<InstanceData>,
    last_frame_count: u32,
    has_changes: bool,
}

impl ModelInstanceGroup {
    fn new() -> Self {
        Self {
            instances: Vec::new(),
            last_frame_count: 0,
            has_changes: false,
        }
    }

    fn clear(&mut self) {
        self.instances.clear();
        self.has_changes = false;
    }

    fn add_instance(&mut self, instance: InstanceData) {
        self.instances.push(instance);
        self.has_changes = true;
    }

    fn mark_changed(&mut self) {
        self.has_changes = true;
    }

    fn finalize_frame(&mut self) {
        self.last_frame_count = self.instances.len() as u32;
        self.has_changes = false;
    }

    fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

/// Manages GPU storage buffer for instances
pub struct InstanceStorageBuffer {
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    capacity: u64,
}

impl InstanceStorageBuffer {
    const INITIAL_CAPACITY: u64 = 128;

    pub fn new(device: &wgpu::Device) -> Self {
        let capacity = Self::INITIAL_CAPACITY;
        let buffer = Self::create_buffer(device, capacity);
        let bind_group_layout = Self::create_bind_group_layout(device);
        let bind_group = Self::create_bind_group(device, &bind_group_layout, &buffer);

        Self {
            buffer,
            bind_group_layout,
            bind_group,
            capacity,
        }
    }

    fn create_buffer(device: &wgpu::Device, capacity: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance storage buffer"),
            size: std::mem::size_of::<InstanceData>() as u64 * capacity,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Instance storage bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Instance storage bind group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    fn ensure_capacity(&mut self, device: &wgpu::Device, required_capacity: u64) {
        if required_capacity > self.capacity {
            let new_capacity = (required_capacity * 2).max(Self::INITIAL_CAPACITY);
            self.buffer = Self::create_buffer(device, new_capacity);
            self.bind_group =
                Self::create_bind_group(device, &self.bind_group_layout, &self.buffer);
            self.capacity = new_capacity;
        }
    }

    fn write_all_instances(&self, queue: &wgpu::Queue, instances: &[InstanceData]) {
        if instances.is_empty() {
            return;
        }

        self.write_instances_at_offset(
            queue, instances, 0, // Start at the beginning of the buffer
        );
    }

    fn write_instances_at_offset(
        &self,
        queue: &wgpu::Queue,
        instances: &[InstanceData],
        start_index: u32,
    ) {
        if instances.is_empty() {
            return;
        }

        let offset = (start_index as u64) * std::mem::size_of::<InstanceData>() as u64;
        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(instances));
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum UpdateStrategy {
    /// No changes detected
    None,
    /// Only specific batches need updating
    Incremental { changed_batches: usize },
    /// Complete rebuild required (model set changed)
    FullRebuild,
}

pub struct InstanceManager {
    // Core data structures
    model_groups: HashMap<Id<RenderModel>, ModelInstanceGroup>,
    batches: Vec<InstanceBatch>,

    // GPU resources
    storage_buffer: InstanceStorageBuffer,

    // State tracking
    frame_counter: u64,
    last_batch_layout_hash: u64,
}

impl InstanceManager {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            model_groups: HashMap::new(),
            batches: Vec::new(),
            storage_buffer: InstanceStorageBuffer::new(device),
            frame_counter: 0,
            last_batch_layout_hash: 0,
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
        self.frame_counter += 1;

        // Phase 1: Gather instances from scene
        self.gather_instances_from_scene(scene);

        // Phase 2: Determine what needs updating
        let update_strategy = self.determine_update_strategy(); // Phase 3: Update GPU data based on strategy
        self.update_gpu_data(update_strategy, device, queue);

        // ImGui debug window
        self.render_debug_window(imgui_ui, update_strategy);
    }

    /// Collect instances from the scene
    fn gather_instances_from_scene(&mut self, scene: &Scene) {
        // Clear previous frame data
        for group in self.model_groups.values_mut() {
            group.clear();
        }

        // Collect instances from scene objects
        for (_, object) in scene.objects.iter() {
            if let Some(model_id) = object.model_id {
                if let Some(model) = scene.models.get(model_id) {
                    if let Some(render_model_id) = model.render_model {
                        let group = self
                            .model_groups
                            .entry(render_model_id)
                            .or_insert_with(ModelInstanceGroup::new);

                        let transform_matrix = *object.transform.get_world_matrix();
                        let instance = InstanceData::new(transform_matrix);
                        group.add_instance(instance);

                        // Mark as changed if transform changed
                        if object.transform.has_changed() {
                            group.mark_changed();
                        }
                    }
                }
            }
        }

        // Remove empty groups (models no longer in scene)
        self.model_groups.retain(|_, group| !group.is_empty());
    }

    /// Determine what kind of update is needed
    fn determine_update_strategy(&mut self) -> UpdateStrategy {
        let current_layout_hash = self.calculate_batch_layout_hash();

        // Check if the overall batch layout changed
        if current_layout_hash != self.last_batch_layout_hash {
            self.last_batch_layout_hash = current_layout_hash;
            return UpdateStrategy::FullRebuild;
        }

        // Count changed batches
        let changed_count = self
            .model_groups
            .values()
            .filter(|group| group.has_changes)
            .count();

        if changed_count == 0 {
            UpdateStrategy::None
        } else {
            UpdateStrategy::Incremental {
                changed_batches: changed_count,
            }
        }
    }

    /// Calculate a hash representing the current batch layout
    fn calculate_batch_layout_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash the set of active models and their instance counts
        let mut models: Vec<_> = self.model_groups.iter().collect();
        models.sort_by_key(|(id, _)| id.index());

        for (model_id, group) in models {
            model_id.index().hash(&mut hasher);
            group.instances.len().hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Update GPU data based on the determined strategy
    fn update_gpu_data(
        &mut self,
        strategy: UpdateStrategy,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        match strategy {
            UpdateStrategy::None => {
                // No changes, nothing to do
            }
            UpdateStrategy::FullRebuild => {
                self.rebuild_batches_and_upload_all(device, queue);
            }
            UpdateStrategy::Incremental { .. } => {
                self.upload_changed_batches(queue);
            }
        }

        // Finalize frame for all groups
        for group in self.model_groups.values_mut() {
            group.finalize_frame();
        }
    }

    /// Rebuild all batches and upload everything to GPU
    fn rebuild_batches_and_upload_all(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.batches.clear();
        let mut current_index = 0u32;

        // Sort models for consistent batch ordering
        let mut models: Vec<_> = self.model_groups.iter().collect();
        models.sort_by_key(|(id, _)| id.index());

        // Build new batches
        for (&model_id, group) in models {
            if !group.instances.is_empty() {
                self.batches.push(InstanceBatch {
                    render_model_id: model_id,
                    start_index: current_index,
                    instance_count: group.instances.len() as u32,
                });
                current_index += group.instances.len() as u32;
            }
        }

        // Collect all instances in batch order
        let total_instances = current_index as usize;
        let mut all_instances = Vec::with_capacity(total_instances);

        for batch in &self.batches {
            if let Some(group) = self.model_groups.get(&batch.render_model_id) {
                all_instances.extend_from_slice(&group.instances);
            }
        }

        // Ensure GPU buffer capacity and upload
        self.storage_buffer
            .ensure_capacity(device, total_instances as u64);
        self.storage_buffer
            .write_all_instances(queue, &all_instances);
    }

    /// Upload only changed batches to GPU
    fn upload_changed_batches(&self, queue: &wgpu::Queue) {
        for batch in &self.batches {
            if let Some(group) = self.model_groups.get(&batch.render_model_id) {
                if group.has_changes {
                    self.storage_buffer.write_instances_at_offset(
                        queue,
                        &group.instances,
                        batch.start_index,
                    );
                }
            }
        }
    }

    /// Get the bind group for rendering
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        self.storage_buffer.bind_group()
    }

    /// Get current batches for rendering
    pub fn batches(&self) -> &[InstanceBatch] {
        &self.batches
    }

    /// Render ImGui debug window with detailed statistics
    fn render_debug_window(&self, imgui_ui: &mut imgui::Ui, update_strategy: UpdateStrategy) {
        imgui_ui
            .window("Instance Manager Debug")
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .size([400.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                // General statistics
                imgui_ui.text("=== General Stats ===");
                imgui_ui.text(format!("Frame: {}", self.frame_counter));
                imgui_ui.text(format!("Models: {}", self.model_groups.len()));
                imgui_ui.text(format!("Batches: {}", self.batches.len()));

                let total_instances: usize = self
                    .model_groups
                    .values()
                    .map(|group| group.instances.len())
                    .sum();
                imgui_ui.text(format!("Total Instances: {}", total_instances));
                imgui_ui.separator();

                // Update strategy information
                imgui_ui.text("=== Update Strategy ===");
                match update_strategy {
                    UpdateStrategy::None => {
                        imgui_ui.text_colored([0.0, 1.0, 0.0, 1.0], "No changes");
                    }
                    UpdateStrategy::Incremental { changed_batches } => {
                        imgui_ui.text_colored(
                            [1.0, 1.0, 0.0, 1.0],
                            format!("Incremental: {} batches changed", changed_batches),
                        );
                    }
                    UpdateStrategy::FullRebuild => {
                        imgui_ui.text_colored([1.0, 0.5, 0.0, 1.0], "Full rebuild");
                    }
                }
                imgui_ui.separator();

                // GPU buffer statistics
                imgui_ui.text("=== GPU Buffer ===");
                imgui_ui.text(format!(
                    "Capacity: {} instances",
                    self.storage_buffer.capacity
                ));
                let buffer_size_mb = (self.storage_buffer.capacity as f64
                    * std::mem::size_of::<InstanceData>() as f64)
                    / (1024.0 * 1024.0);
                imgui_ui.text(format!("Buffer Size: {:.2} MB", buffer_size_mb));

                let usage_percent = if self.storage_buffer.capacity > 0 {
                    (total_instances as f64 / self.storage_buffer.capacity as f64) * 100.0
                } else {
                    0.0
                };
                imgui_ui.text(format!("Usage: {:.1}%", usage_percent));
                imgui_ui.separator();

                // Per-model breakdown
                imgui_ui.text("=== Per-Model Breakdown ===");
                if imgui_ui.collapsing_header("Model Details", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    // Sort models by ID for consistent display
                    let mut models: Vec<_> = self.model_groups.iter().collect();
                    models.sort_by_key(|(id, _)| id.index());

                    for (&model_id, group) in models {
                        let changed_indicator = if group.has_changes { " *" } else { "" };
                        imgui_ui.text(format!(
                            "Model {}: {} instances{}",
                            model_id.index(),
                            group.instances.len(),
                            changed_indicator
                        ));
                    }
                }
                imgui_ui.separator();

                // Batch information
                imgui_ui.text("=== Batch Layout ===");
                if imgui_ui.collapsing_header("Batch Details", imgui::TreeNodeFlags::empty()) {
                    imgui_ui.text(format!(
                        "Layout Hash: 0x{:016x}",
                        self.last_batch_layout_hash
                    ));

                    for (i, batch) in self.batches.iter().enumerate() {
                        imgui_ui.text(format!(
                            "Batch {}: Model {} [{}-{}] ({} instances)",
                            i,
                            batch.render_model_id.index(),
                            batch.start_index,
                            batch.start_index + batch.instance_count - 1,
                            batch.instance_count
                        ));
                    }
                }
            });
    }
}

pub fn render_batch(
    render_pass: &mut wgpu::RenderPass<'_>,
    render_model: &RenderModel,
    batch: &InstanceBatch,
) {
    for primitive in render_model.primitives.iter() {
        render_pass.set_vertex_buffer(0, primitive.vertex_buffer.slice(..));
        render_pass.set_index_buffer(primitive.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(
            0..primitive.num_indices,
            0,
            batch.start_index..(batch.start_index + batch.instance_count),
        );
    }
}
