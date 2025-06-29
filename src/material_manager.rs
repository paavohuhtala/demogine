use std::collections::HashMap;

use id_arena::{Arena, Id};

use crate::asset_pipeline::materials::PbrMaterialData;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GltfMaterialKey {
    pub file_name: String,
    pub material_name: String,
}

pub struct MaterialManager {
    materials: Arena<PbrMaterialData>,
    materials_by_gltf: HashMap<GltfMaterialKey, Id<PbrMaterialData>>,
}

pub type MaterialId = Id<PbrMaterialData>;

impl MaterialManager {
    pub fn new() -> Self {
        Self {
            materials: Arena::new(),
            materials_by_gltf: HashMap::new(),
        }
    }

    pub fn add_material(&mut self, material_data: PbrMaterialData) -> Id<PbrMaterialData> {
        let id = self.materials.alloc(material_data);
        id
    }

    pub fn get_gltf_material(
        &self,
        file_name: &str,
        material_name: &str,
    ) -> Option<Id<PbrMaterialData>> {
        let key = GltfMaterialKey {
            file_name: file_name.to_string(),
            material_name: material_name.to_string(),
        };
        self.materials_by_gltf.get(&key).cloned()
    }

    pub fn load_all_materials_from_gltf(
        &mut self,
        file_name: &str,
        document: &gltf::Document,
        images: &mut [gltf::image::Data],
    ) {
        for material in document.materials() {
            let material_name = material.name().unwrap_or("Unnamed material");

            let key = GltfMaterialKey {
                file_name: file_name.to_string(),
                material_name: material_name.to_string(),
            };

            if self.materials_by_gltf.contains_key(&key) {
                continue;
            }

            let base_color = material.pbr_metallic_roughness().base_color_texture();
            let normal = material.normal_texture();
            // The GLTF spec defines separate occlusion and metallic roughness textures,
            // but Substance packs all three into a single occlusionRoughnessMetallic texture.
            let ao_roughness_metallic = material.occlusion_texture();

            // Remove textures when found and replace with default using swap
            let default_texture = gltf::image::Data {
                pixels: Vec::new(),
                format: gltf::image::Format::R8G8B8,
                width: 0,
                height: 0,
            };

            let base_color = base_color.map(|texture_info| {
                let texture_index = texture_info.texture().index();
                let mut texture = default_texture.clone();
                std::mem::swap(
                    &mut texture,
                    images
                        .get_mut(texture_index)
                        .expect("GLTF texture index out of bounds: baseColor"),
                );
                texture = convert_image_data_to_rgba(texture);
                texture
            });

            let normal = normal.map(|texture_info| {
                let texture_index = texture_info.texture().index();
                let mut texture = default_texture.clone();
                std::mem::swap(
                    &mut texture,
                    images
                        .get_mut(texture_index)
                        .expect("GLTF texture index out of bounds: normal"),
                );
                texture = convert_image_data_to_rgba(texture);
                texture
            });

            let ao_roughness_metallic = ao_roughness_metallic.map(|texture_info| {
                let texture_index = texture_info.texture().index();
                let mut texture = default_texture.clone();
                std::mem::swap(
                    &mut texture,
                    images
                        .get_mut(texture_index)
                        .expect("GLTF texture index out of bounds: occlusionRoughnessMetallic"),
                );
                texture = convert_image_data_to_rgba(texture);
                texture
            });

            let material_data = PbrMaterialData {
                name: material_name.to_string(),
                base_color,
                normal,
                ao_roughness_metallic,
            };

            let id = self.add_material(material_data);
            self.materials_by_gltf.insert(key, id);
        }
    }

    pub fn materials(&self) -> impl Iterator<Item = &PbrMaterialData> {
        self.materials.iter().map(|(_, material)| material)
    }

    pub fn draw_ui(&self, ui: &imgui::Ui) {
        ui.window("Material manager").build(|| {
            ui.text("Materials:");
            ui.separator();

            for (id, material) in self.materials.iter() {
                ui.text(format!("{}: Name: {}", id.index(), material.name,));
            }
        });
    }
}

fn convert_image_data_to_rgba(data: gltf::image::Data) -> gltf::image::Data {
    if data.format == gltf::image::Format::R8G8B8A8 {
        return data;
    }

    if data.format != gltf::image::Format::R8G8B8 {
        panic!("Unsupported image format: {:?}", data.format);
    }

    let mut rgba_data = Vec::with_capacity(data.pixels.len() * 4);

    for pixel in data.pixels.chunks(3) {
        rgba_data.extend_from_slice(pixel);
        rgba_data.push(255);
    }

    gltf::image::Data {
        pixels: rgba_data,
        format: gltf::image::Format::R8G8B8A8,
        width: data.width,
        height: data.height,
    }
}
