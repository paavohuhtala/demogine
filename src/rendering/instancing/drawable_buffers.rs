use crate::rendering::instancing::{drawable_storage_buffer::DrawableBuffer, MAX_DRAWABLES};

#[derive(Clone)]
pub struct DrawableBuffers {
    pub all_drawables: DrawableBuffer,
    pub visible_drawables: DrawableBuffer,
}

impl DrawableBuffers {
    pub fn new(device: &wgpu::Device, initial_capacity: u64) -> Self {
        let all_drawables = DrawableBuffer::new(device, initial_capacity);
        let visible_drawables = DrawableBuffer::new(device, initial_capacity);

        Self {
            all_drawables,
            visible_drawables,
        }
    }

    pub fn new_default_capacity(device: &wgpu::Device) -> Self {
        Self::new(device, MAX_DRAWABLES as u64)
    }
}
