mod instance_data;
mod instance_group;
mod instance_manager;
mod instance_storage_buffer;

pub use instance_manager::InstanceManager;

use crate::rendering::instancing::instance_group::InstanceBatch;
use crate::rendering::render_model::RenderModel;

/// Defines whether an instance is static (rarely changes) or dynamic (frequently updated)
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstanceType {
    /// Dynamic instances that change frequently (e.g., moving objects, animated elements)
    #[default]
    Dynamic,
    /// Static instances that rarely change (e.g., level geometry, buildings)
    Static,
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

/// Get the appropriate bind group for a batch
pub fn get_bind_group_for_batch<'a>(
    instance_manager: &'a InstanceManager,
    batch: &InstanceBatch,
) -> &'a wgpu::BindGroup {
    match batch.instance_type {
        InstanceType::Static => instance_manager.static_bind_group(),
        InstanceType::Dynamic => instance_manager.dynamic_bind_group(),
    }
}
