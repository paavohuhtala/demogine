#define_import_path shared::drawable

struct Drawable {
    model_matrix: mat4x4<f32>,
    primitive_index: u32,
    material_id: u32,
    padding: array<u32, 2>,
}
