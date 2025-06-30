use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use crate::rendering::util::bind_group_builder::BindGroupBuilder;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GlobalUniformState {
    pub resolution: [f32; 2],
    pub now: f32,
    _padding: f32,
}

impl GlobalUniformState {
    pub fn new(resolution: PhysicalSize<u32>, now: f32) -> Self {
        Self {
            resolution: [resolution.width as f32, resolution.height as f32],
            now,
            _padding: 0.0,
        }
    }
}

pub struct GlobalUniform {
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl GlobalUniform {
    pub fn new(device: &wgpu::Device, initial_state: GlobalUniformState) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Global uniform buffer"),
            contents: bytemuck::cast_slice(&[initial_state]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let (bind_group_layout, bind_group) =
            BindGroupBuilder::new("Global uniform", wgpu::ShaderStages::VERTEX_FRAGMENT)
                .uniform(
                    0,
                    "Global uniform buffer",
                    wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                )
                .build(device);

        Self {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, state: GlobalUniformState) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[state]));
    }
}
