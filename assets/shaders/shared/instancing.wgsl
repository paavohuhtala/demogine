#define_import_path shared::instancing

/// Instance data structure that matches the Rust InstanceData struct
/// Contains transform matrix and AABB bounds for each instance
struct InstanceData {
    model_matrix: mat4x4<f32>,
    aabb_min: vec4<f32>,  // W component unused but required for alignment
    aabb_max: vec4<f32>,  // W component unused but required for alignment
}

struct Drawable {
    model_matrix: mat4x4<f32>,
    primitive_index: u32,
    padding: array<u32, 3>,
}
