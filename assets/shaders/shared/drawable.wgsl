#define_import_path shared::drawable

struct InputDrawable {
    model_matrix: mat4x4<f32>,
    inverse_transpose_model_matrix: mat4x4<f32>,
    mesh_index: u32,
    material_id: u32,
    padding: array<u32, 2>,
}

// This is just a copy, for now
struct VisibleDrawable {
    model_matrix: mat4x4<f32>,
    inverse_transpose_model_matrix: mat4x4<f32>,
    mesh_index: u32,
    material_id: u32,
    padding: array<u32, 2>,
}
