// Blender's GLTF exporter generates tangents.
// So does Substance Painter, except not until the 2022 version (which I don't have).
// To avoid having to export from both programs, we'll just generate tangents here.

use anyhow::bail;
use bevy_mikktspace::{generate_tangents, Geometry};

use crate::model::ModelPrimitive;

impl Geometry for ModelPrimitive {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        let vertex = self.vertex_by_triangle_index(face, vert);
        vertex.position.to_array()
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        let vertex = self.vertex_by_triangle_index(face, vert);
        vertex.normal.to_array()
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        let vertex = self.vertex_by_triangle_index(face, vert);
        vertex.tex_coords.to_array()
    }

    fn set_tangent(
        &mut self,
        tangent: [f32; 3],
        _bi_tangent: [f32; 3],
        _f_mag_s: f32,
        _f_mag_t: f32,
        _bi_tangent_preserves_orientation: bool,
        face: usize,
        vert: usize,
    ) {
        let vertex = self.vertex_by_triangle_index_mut(face, vert);
        vertex.tangent = glam::Vec3::from_array(tangent);
    }
}

impl ModelPrimitive {
    /// Generate tangents for this primitive using mikktspace algorithm
    pub fn generate_tangents(&mut self) -> anyhow::Result<()> {
        let success = generate_tangents(self);

        if !success {
            bail!("Failed to generate tangents")
        }

        Ok(())
    }
}
