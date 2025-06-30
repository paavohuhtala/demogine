use wgpu::{BufferUsages, ShaderStages};

use crate::rendering::{
    instancing::drawable::Drawable, util::bind_group_builder::BindGroupBuilder,
};

#[derive(Clone)]
pub struct DrawableBuffer {
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl DrawableBuffer {
    pub fn new(device: &wgpu::Device, initial_capacity: u64) -> Self {
        let buffer = Self::create_buffer(device, initial_capacity);
        let (bind_group_layout, bind_group) = BindGroupBuilder::new(
            "Drawable storage",
            ShaderStages::VERTEX_FRAGMENT | ShaderStages::COMPUTE,
        )
        .storage_r(0, "Drawable storage buffer", buffer.as_entire_binding())
        .build(device);

        Self {
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    fn create_buffer(device: &wgpu::Device, capacity: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Drawable storage buffer"),
            size: std::mem::size_of::<Drawable>() as u64 * capacity,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn write_drawables_at_offset(
        &self,
        queue: &wgpu::Queue,
        instances: &[Drawable],
        start_index: u32,
    ) {
        if instances.is_empty() {
            return;
        }

        let offset = (start_index as u64) * std::mem::size_of::<Drawable>() as u64;
        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(instances));
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
