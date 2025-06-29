use glam::Mat4;

/// This should match the same structure defined in WGSL
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Drawable {
    pub model_matrix: Mat4,
    pub primitive_index: u32,
    pub material_id: u32,
    _padding: [u32; 2],
}

impl Drawable {
    pub fn new(model_matrix: Mat4, primitive_index: u32, material_id: u32) -> Self {
        Self {
            model_matrix,
            primitive_index,
            material_id,
            _padding: [0; 2],
        }
    }
}
