#define_import_path shared::mesh_info

struct MeshInfo {
    index_count: u32,
    first_index: u32,
    vertex_offset: u32,
    _padding: u32,
    aabb_min: vec4<f32>,
    aabb_max: vec4<f32>,
}
