use glam::Mat4;

/// This should match the same structure defined in WGSL
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Drawable {
    pub model_matrix: Mat4,
    pub primitive_index: u32,
    _padding: [u32; 3],
}

impl Drawable {
    pub fn new(model_matrix: Mat4, primitive_index: u32) -> Self {
        Self {
            model_matrix,
            primitive_index,
            _padding: [0; 3],
        }
    }
}
