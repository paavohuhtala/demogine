use wgpu::BufferUsages;

use crate::model::Instance;

pub struct Instances {
    instances: Vec<Instance>,
}

impl Instances {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn add(&mut self, instance: Instance) {
        self.instances.push(instance);
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    pub fn write_to_buffer(&self, queue: &wgpu::Queue, instance_buffer: &InstanceBuffer) {
        queue.write_buffer(
            &instance_buffer.buffer(),
            0,
            bytemuck::cast_slice(&self.instances),
        );
    }

    pub fn should_render(&self) -> bool {
        !self.instances.is_empty()
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }
}

pub struct InstanceBuffer(wgpu::Buffer);

impl InstanceBuffer {
    const MAX_INSTANCES: u64 = 128;

    pub fn new(device: &wgpu::Device, name: impl Into<String>) -> Self {
        let name: String = name.into();

        let descriptor = Self::descriptor(&name);
        let buffer = device.create_buffer(&descriptor);

        Self(buffer)
    }

    fn descriptor(name: &str) -> wgpu::BufferDescriptor<'static> {
        // Damned lifetimes! Nothing a nice controlled memory leak can't fix.
        let label = format!("Instance buffer ({})", name);
        let label = label.into_boxed_str();
        let label = Box::leak(label);

        wgpu::BufferDescriptor {
            label: Some(label),
            size: std::mem::size_of::<Instance>() as u64 * Self::MAX_INSTANCES,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.0
    }

    pub fn bind(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        // TODO make slot configurable if there's ever a second type of instance buffer
        render_pass.set_vertex_buffer(1, self.buffer().slice(..));
    }
}
