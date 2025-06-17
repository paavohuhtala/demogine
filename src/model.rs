use std::mem::offset_of;

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use gltf::buffer;
use itertools::izip;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position: Vec3,
    normal: Vec3,
    tex_coords: Vec2,
}

pub struct Model {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Model {
    pub fn from_gtlf(mesh: gltf::Mesh, buffers: &[buffer::Data]) -> anyhow::Result<Model> {
        // Meshes with multiple primitives are not supported

        let primitive = mesh.primitives().next().context("No primitives in mesh")?;

        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        let position_reader = reader.read_positions().expect("Failed to read positions");
        let normal_reader = reader.read_normals().expect("Failed to read normals");
        let tex_coords_reader = reader
            .read_tex_coords(0)
            .expect("Failed to read tex coords")
            .into_f32();

        let vertices = izip!(position_reader, normal_reader, tex_coords_reader)
            .map(|(pos, normal, tex_coords)| Vertex {
                position: Vec3::from(pos),
                normal: Vec3::from(normal),
                tex_coords: Vec2::from(tex_coords),
            })
            .collect::<Vec<Vertex>>();

        let index_reader = reader.read_indices().expect("Failed to read indices");
        let indices = index_reader.into_u32().collect::<Vec<u32>>();

        Ok(Model { vertices, indices })
    }

    pub fn quad() -> Model {
        let vertices = vec![
            Vertex {
                position: Vec3::new(-0.5, -0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
                tex_coords: Vec2::new(0.0, 1.0),
            },
            Vertex {
                position: Vec3::new(0.5, -0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
                tex_coords: Vec2::new(1.0, 1.0),
            },
            Vertex {
                position: Vec3::new(0.5, 0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
                tex_coords: Vec2::new(1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(-0.5, 0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
                tex_coords: Vec2::new(0.0, 0.0),
            },
        ];

        let indices = vec![2, 1, 3, 2, 3, 0];

        Model { vertices, indices }
    }
}

pub struct RenderModel {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl RenderModel {
    pub fn from_model(device: &wgpu::Device, model: Model) -> Self {
        println!("Creating render model");
        println!("Vertices: {:?}", model.vertices.len());
        println!("Indices: {:?}", model.indices.len());

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&model.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&model.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        RenderModel {
            vertex_buffer,
            index_buffer,
            num_indices: model.indices.len() as u32,
        }
    }
}

pub const RENDER_MODEL_VBL: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: offset_of!(Vertex, normal) as wgpu::BufferAddress,
            shader_location: 1,
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
