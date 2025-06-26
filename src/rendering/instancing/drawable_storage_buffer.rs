use wgpu::{BindingType, BufferBindingType, BufferUsages, ShaderStages};

use crate::rendering::instancing::drawable::Drawable;

pub struct DrawableStorageBuffer {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl DrawableStorageBuffer {
    pub fn new(device: &wgpu::Device, initial_capacity: u64) -> Self {
        let buffer = Self::create_buffer(device, initial_capacity);
        let bind_group_layout = Self::create_bind_group_layout(device);
        let bind_group = Self::create_bind_group(device, &bind_group_layout, &buffer);

        Self { buffer, bind_group }
    }

    fn create_buffer(device: &wgpu::Device, capacity: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Drawable storage buffer"),
            size: std::mem::size_of::<Drawable>() as u64 * capacity,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Drawable storage bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
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
            label: Some("Drawable storage bind group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
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

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}
