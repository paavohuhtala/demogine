#define_import_path shared::material_info

struct MaterialInfo {
    base_color: u32,
    normal: u32,
    ao_roughness_metallic: u32,
    _padding: u32,
}
