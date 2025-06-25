use wgpu::{BindingType, BufferBindingType, BufferUsages, ShaderStages};

use crate::rendering::instancing::instance_data::InstanceData;

/// Manages GPU storage buffer for instances
pub struct InstanceStorageBuffer {
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    capacity: u64,
}

impl InstanceStorageBuffer {
    pub fn new(device: &wgpu::Device, initial_capacity: u64) -> Self {
        let buffer = Self::create_buffer(device, initial_capacity);
        let bind_group_layout = Self::create_bind_group_layout(device);
        let bind_group = Self::create_bind_group(device, &bind_group_layout, &buffer);

        Self {
            buffer,
            bind_group_layout,
            bind_group,
            capacity: initial_capacity,
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
            label: Some("Instance storage bind group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    pub fn ensure_capacity(&mut self, device: &wgpu::Device, required_capacity: u64) {
        if required_capacity > self.capacity {
            let new_capacity = required_capacity * 2;
            self.buffer = Self::create_buffer(device, new_capacity);
            self.bind_group =
                Self::create_bind_group(device, &self.bind_group_layout, &self.buffer);
            self.capacity = new_capacity;
        }
    }

    pub fn write_instances_at_offset(
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

    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}
