use std::mem::offset_of;

use id_arena::Id;
use wgpu::util::DeviceExt;

use crate::model::{Model, ModelPrimitive, Vertex};

pub type RenderModelId = Id<RenderModel>;

pub struct RenderPrimitive {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl RenderPrimitive {
    fn from_primitive(device: &wgpu::Device, model: &Model, primitive: &ModelPrimitive) -> Self {
        let vertex_buffer_name = format!(
            "Vertex buffer ({}, primitive {})",
            model.name, primitive.index
        );
        let index_buffer_name = format!(
            "Index buffer ({}, primitive {})",
            model.name, primitive.index
        );

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&vertex_buffer_name),
            contents: bytemuck::cast_slice(&primitive.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&index_buffer_name),
            contents: bytemuck::cast_slice(&primitive.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: primitive.indices.len() as u32,
        }
    }
}

pub struct RenderModel {
    pub primitives: Vec<RenderPrimitive>,
}

impl RenderModel {
    pub fn from_model(device: &wgpu::Device, model: &Model) -> Self {
        let primitives = model
            .primitives
            .iter()
            .map(|primitive| RenderPrimitive::from_primitive(device, model, primitive))
            .collect();

        RenderModel { primitives }
    }
}

pub const RENDER_MODEL_VBL: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute {
            offset: offset_of!(Vertex, position) as wgpu::BufferAddress,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: offset_of!(Vertex, normal) as wgpu::BufferAddress,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: offset_of!(Vertex, tex_coords) as wgpu::BufferAddress,
            shader_location: 2,
            format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
            offset: offset_of!(Vertex, tangent) as wgpu::BufferAddress,
            shader_location: 3,
            format: wgpu::VertexFormat::Float32x3,
        },
    ],
};
