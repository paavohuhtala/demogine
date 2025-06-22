use std::mem::offset_of;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3, Vec4, Vec4Swizzles};
use gltf::buffer;
use id_arena::Id;
use itertools::izip;
use wgpu::util::DeviceExt;

use crate::rendering::instance::InstanceBuffer;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position: Vec3,
    normal: Vec3,
    tex_coords: Vec2,
    tangent: Vec3,
}

pub struct ModelPrimitive {
    pub index: usize,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub struct Model {
    pub name: String,
    pub primitives: Vec<ModelPrimitive>,
}

pub type Buffers<'a> = &'a [buffer::Data];

impl Model {
    pub fn from_gltf(
        name: impl Into<String>,
        mesh: gltf::Mesh,
        buffers: Buffers,
    ) -> anyhow::Result<Model> {
        let mut model = Model {
            name: name.into(),
            primitives: Vec::new(),
        };

        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                return Err(anyhow::anyhow!(
                    "Unsupported primitive mode: {:?}",
                    primitive.mode()
                ));
            }

            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let position_reader = reader.read_positions().expect("Failed to read positions");
            let normal_reader = reader.read_normals().expect("Failed to read normals");
            let tex_coords_reader = reader
                .read_tex_coords(0)
                .expect("Failed to read tex coords")
                .into_f32();
            let tangent_reader = reader.read_tangents().expect("Failed to read tangents");

            let vertices = izip!(
                position_reader,
                normal_reader,
                tex_coords_reader,
                tangent_reader
            )
            .map(|(pos, normal, tex_coords, tangent)| Vertex {
                position: Vec3::from(pos),
                normal: Vec3::from(normal),
                tex_coords: Vec2::from(tex_coords),
                tangent: Vec4::from(tangent).xyz(),
            })
            .collect::<Vec<Vertex>>();

            let index_reader = reader.read_indices().expect("Failed to read indices");
            let indices = index_reader.into_u32().collect::<Vec<u32>>();

            model.primitives.push(ModelPrimitive {
                index: primitive.index(),
                vertices,
                indices,
            });
        }

        if model.primitives.is_empty() {
            return Err(anyhow::anyhow!("Mesh without primitives: {}", model.name));
        }

        Ok(model)
    }
}

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
    pub instance_buffer: InstanceBuffer,
}

impl RenderModel {
    pub fn from_model(device: &wgpu::Device, model: &Model) -> Self {
        let primitives = model
            .primitives
            .iter()
            .map(|primitive| RenderPrimitive::from_primitive(device, model, primitive))
            .collect();
        let instance_buffer = InstanceBuffer::new(device, model.name.clone());

        RenderModel {
            primitives,
            instance_buffer,
        }
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
