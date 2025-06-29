#[derive(Debug, Clone)]
pub struct PbrMaterialData {
    pub name: String,
    pub base_color: Option<gltf::image::Data>,
    pub normal: Option<gltf::image::Data>,
    pub ao_roughness_metallic: Option<gltf::image::Data>,
}
