mod draw_command_generator;
mod drawable;
mod drawable_buffers;
mod drawable_manager;
mod drawable_storage_buffer;

pub use drawable_buffers::DrawableBuffers;
pub use drawable_manager::DrawableManager;

pub const MAX_MESHES: usize = 128;
pub const MAX_DRAWABLES: usize = 32_000;

/// Defines whether an instance is static (rarely changes) or dynamic (frequently updated)
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstanceType {
    /// Dynamic instances that change frequently (e.g., moving objects, animated elements)
    #[default]
    Dynamic,
    /// Static instances that rarely change (e.g., level geometry, buildings)
    Static,
}
