use glam::Mat4;
use id_arena::Arena;
use wgpu::BufferUsages;

use crate::{rendering::render_model::RenderModel, scene_graph::scene::Scene};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub model: Mat4,
}

impl Instance {
    pub fn descriptor() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

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

fn clear_instances(render_models: &mut Arena<RenderModel>) {
    for (_, model) in render_models.iter_mut() {
        model.clear_instances();
    }
}

pub fn gather_instances(scene: &Scene, render_models: &mut Arena<RenderModel>) {
    clear_instances(render_models);

    // Iterate through all objects and collect instances for each model
    for (_, object) in scene.objects.iter() {
        if let Some(model_id) = object.model_id {
            // Get the world transformation matrix from the object's transform
            let transform_matrix = *object.transform.get_world_matrix();

            let instance = Instance {
                model: transform_matrix,
            };

            // Add the instance to the corresponding model
            if let Some(model) = scene.models.get(model_id) {
                let render_model_id = model
                    .render_model
                    .expect("Model should have a render model");

                let render_model = render_models
                    .get_mut(render_model_id)
                    .expect("Model not found in render models");
                render_model.add_instance(instance);
            }
        }
    }
}
