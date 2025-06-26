use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Vec4, Vec4Swizzles};
use gltf::buffer;
use itertools::izip;

use crate::math::bounds::AABB;

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

            // Extract bounding box from GLTF primitive
            let bounding_box = {
                let bounds = primitive.bounding_box();
                let min = Vec3::from(bounds.min);
                let max = Vec3::from(bounds.max);
                AABB::new(min, max)
            };

            let global_index = *primitive_index;

            model.primitives.push(ModelPrimitive {
                vertices,
                indices,
                bounding_box,
                global_index,
            });

            *primitive_index += 1;
        }

        if model.primitives.is_empty() {
            return Err(anyhow::anyhow!("Mesh without primitives: {}", model.name));
        }

        Ok(model)
    }
}
