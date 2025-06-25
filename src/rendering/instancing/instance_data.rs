use glam::Mat4;

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
