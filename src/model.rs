use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use gltf::buffer;
use itertools::izip;

use crate::{
    material_manager::{MaterialId, MaterialManager},
    math::bounds::AABB,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coords: Vec2,
    pub tangent: Vec3,
}

pub struct ModelPrimitive {
    pub global_index: usize,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub bounding_box: AABB,
    pub material_id: MaterialId,
}

impl ModelPrimitive {
    pub fn vertex_by_triangle_index(&self, triangle_index: usize, vertex_index: usize) -> &Vertex {
        let vertex_offset = triangle_index * 3 + vertex_index;
        &self.vertices[self.indices[vertex_offset] as usize]
    }

    pub fn vertex_by_triangle_index_mut(
        &mut self,
        triangle_index: usize,
        vertex_index: usize,
    ) -> &mut Vertex {
        let vertex_offset = triangle_index * 3 + vertex_index;
        &mut self.vertices[self.indices[vertex_offset] as usize]
    }
}

pub struct Model {
    pub name: String,
    pub primitives: Vec<ModelPrimitive>,
}

pub type Buffers<'a> = &'a [buffer::Data];

impl Model {
    pub fn from_gltf(
        material_manager: &MaterialManager,
        file_name: &str,
        name: impl Into<String>,
        mesh: gltf::Mesh,
        buffers: Buffers,
        primitive_index: &mut usize,
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

            let material_name = primitive
                .material()
                .name()
                .expect("Material name is required");
            let material_id = material_manager
                .get_gltf_material(file_name, material_name)
                .with_context(|| {
                    format!(
                        "Failed to find material '{}' for model '{}'",
                        material_name, model.name
                    )
                })?;

            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let position_reader = reader.read_positions().expect("Failed to read positions");
            let normal_reader = reader.read_normals().expect("Failed to read normals");
            let tex_coords_reader = reader
                .read_tex_coords(0)
                .expect("Failed to read tex coords")
                .into_f32();

            let vertices = izip!(position_reader, normal_reader, tex_coords_reader,)
                .map(|(pos, normal, tex_coords)| Vertex {
                    position: Vec3::from(pos),
                    normal: Vec3::from(normal),
                    tex_coords: Vec2::from(tex_coords),
                    tangent: Vec3::ZERO,
                })
                .collect::<Vec<Vertex>>();

            let index_reader = reader.read_indices().expect("Failed to read indices");
            let indices = index_reader.into_u32().collect::<Vec<u32>>();

            // Extract bounding box from GLTF primitive
            let bounding_box = {
                let bounds = primitive.bounding_box();
                let min = Vec3::from(bounds.min);
                let max = Vec3::from(bounds.max);
                AABB::new(min, max)
            };

            let global_index = *primitive_index;

            let mut primitive = ModelPrimitive {
                vertices,
                indices,
                bounding_box,
                global_index,
                material_id,
            };

            primitive.generate_tangents().with_context(|| {
                format!(
                    "Failed to generate tangents for primitive in model '{}'",
                    model.name
                )
            })?;

            model.primitives.push(primitive);
            *primitive_index += 1;
        }

        if model.primitives.is_empty() {
            return Err(anyhow::anyhow!("Mesh without primitives: {}", model.name));
        }

        Ok(model)
    }
}
