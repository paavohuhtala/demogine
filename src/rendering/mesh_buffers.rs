use wgpu::util::DeviceExt;

use crate::asset_pipeline::mesh_baker::BakedMeshes;

pub struct MeshBuffers {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub meshes: wgpu::Buffer,
}

impl MeshBuffers {
    pub fn new(device: &wgpu::Device, baked_primitives: &BakedMeshes) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex megabuffer"),
            contents: bytemuck::cast_slice(&baked_primitives.buffers.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index megabuffer"),
            contents: bytemuck::cast_slice(&baked_primitives.buffers.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mesh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh megabuffer"),
            contents: bytemuck::cast_slice(&baked_primitives.meshes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        Self {
            vertices: vertex_buffer,
            indices: index_buffer,
            meshes: mesh_buffer,
        }
    }
}
