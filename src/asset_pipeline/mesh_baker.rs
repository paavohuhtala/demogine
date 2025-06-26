use bytemuck::{Pod, Zeroable};
use glam::Vec4;

use crate::model::{Model, Vertex};

pub struct PrimitiveBuffers {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MeshInfo {
    pub index_count: u32,
    pub first_index: u32,
    pub vertex_offset: u32,
    _padding: u32,

    // w coordinates are unused
    pub aabb_min: Vec4,
    pub aabb_max: Vec4,
}

pub struct BakedMeshes {
    pub buffers: PrimitiveBuffers,
    pub meshes: Vec<MeshInfo>,
}

pub fn bake_models(models: &[&Model]) -> BakedMeshes {
    let mut buffers = PrimitiveBuffers {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    let mut primitives = Vec::new();

    for (_, model) in models.iter().enumerate() {
        for (_, primitive) in model.primitives.iter().enumerate() {
            let vertex_offset = buffers.vertices.len() as u32;
            let first_index = buffers.indices.len() as u32;

            buffers.vertices.extend(primitive.vertices.iter());
            buffers.indices.extend(primitive.indices.iter());

            let primitive = MeshInfo {
                first_index,
                index_count: primitive.indices.len() as u32,
                vertex_offset,
                _padding: 0,
                aabb_min: primitive.bounding_box.min.extend(0.0),
                aabb_max: primitive.bounding_box.max.extend(0.0),
            };

            primitives.push(primitive);
        }
    }

    BakedMeshes {
        buffers,
        meshes: primitives,
    }
}
