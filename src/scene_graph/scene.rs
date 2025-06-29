use glam::{Mat4, Quat, Vec3};
use id_arena::Arena;
use std::collections::HashMap;

use crate::material_manager::MaterialManager;
use crate::model::{Buffers, Model};
use crate::rendering::instancing::InstanceType;
use crate::scene_graph::object3d::{Object3D, ObjectId};
use crate::scene_graph::scene_model::{SceneModel, SceneModelId};
use crate::scene_graph::transform::Transform;

pub struct Scene {
    pub objects: Arena<Object3D>,
    pub models: Arena<SceneModel>,
    next_primitive_index: usize,
    gltf_mesh_to_model: HashMap<usize, SceneModelId>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            objects: Arena::new(),
            models: Arena::new(),
            next_primitive_index: 0,
            gltf_mesh_to_model: HashMap::new(),
        }
    }

    pub fn add_object(&mut self, object: Object3D) -> ObjectId {
        self.objects.alloc(object)
    }

    #[allow(dead_code)]
    pub fn get_object(&self, id: ObjectId) -> Option<&Object3D> {
        self.objects.get(id)
    }

    #[allow(dead_code)]
    pub fn get_object_mut(&mut self, id: ObjectId) -> Option<&mut Object3D> {
        self.objects.get_mut(id)
    }

    #[allow(dead_code)]
    pub fn get_object_by_name(&self, name: &str) -> Option<ObjectId> {
        self.objects
            .iter()
            .find(|(_, object)| object.name == name)
            .map(|(id, _)| id)
    }

    pub fn add_model(&mut self, model: SceneModel) -> SceneModelId {
        self.models.alloc(model)
    }

    pub fn spawn_gltf_scene(
        &mut self,
        material_manager: &MaterialManager,
        file_name: &str,
        buffers: Buffers,
        scene: &gltf::Scene,
        instance_type: InstanceType,
    ) -> Option<ObjectId> {
        let mut last_object_id = None;

        for node in scene.nodes() {
            last_object_id = Some(self.spawn_gltf_node(
                material_manager,
                file_name,
                buffers,
                &node,
                None,
                instance_type,
            ));
        }

        last_object_id
    }

    fn spawn_gltf_node(
        &mut self,
        material_manager: &MaterialManager,
        file_name: &str,
        buffers: Buffers,
        node: &gltf::Node,
        parent: Option<ObjectId>,
        instance_type: InstanceType,
    ) -> ObjectId {
        let mut object = Object3D::default();
        let node_name = node.name().unwrap_or("Unnamed").to_string();
        object.name = node_name.clone();
        let (translation, rotation, scale) = node.transform().decomposed();

        object.transform.set_transform(
            translation.into(),
            Quat::from_array(rotation),
            scale[0], // Assume uniform scale for simplicity
        );

        object.instance_type = instance_type;

        if let Some(mesh) = node.mesh() {
            let mesh_index = mesh.index();

            let mesh_id = match self.gltf_mesh_to_model.get(&mesh_index).copied() {
                Some(mesh_id) => mesh_id,
                None => {
                    let mesh_name = mesh
                        .name()
                        .map(String::from)
                        .unwrap_or_else(|| format!("{} (Mesh)", node_name));

                    let model = Model::from_gltf(
                        material_manager,
                        file_name,
                        mesh_name.clone(),
                        mesh,
                        buffers,
                        &mut self.next_primitive_index,
                    )
                    .expect("Failed to create model from glTF mesh");
                    let scene_model = SceneModel::new(model);
                    let mesh_id = self.add_model(scene_model);
                    self.gltf_mesh_to_model.insert(mesh_index, mesh_id);

                    mesh_id
                }
            };

            object.model_id = Some(mesh_id);
        }

        let object_id = self.add_object(object);

        // Set parent-child relationship if there's a parent
        if let Some(parent_id) = parent {
            self.set_object_parent(object_id, Some(parent_id));
        }

        for child in node.children() {
            self.spawn_gltf_node(
                material_manager,
                file_name,
                buffers,
                &child,
                Some(object_id),
                instance_type,
            );
        }

        object_id
    }

    /// Updates all object transforms in hierarchical order
    fn update_transforms(&self) {
        // Find all root objects (objects without parents)
        let root_objects = self.objects.iter().filter_map(|(id, object)| {
            if object.parent_id.is_none() {
                Some(id)
            } else {
                None
            }
        });

        // Update transforms starting from root objects
        for root_id in root_objects {
            self.update_object_transform_recursive(root_id, Mat4::IDENTITY);
        }
    }

    /// Recursively updates an object's world transform and its children
    fn update_object_transform_recursive(&self, object_id: ObjectId, parent_world_matrix: Mat4) {
        if let Some(object) = self.objects.get(object_id) {
            // Only update if the world transform is dirty
            if object.transform.is_world_dirty() {
                let local_matrix = *object.transform.get_local_matrix();
                let world_matrix = parent_world_matrix * local_matrix;
                object.transform.set_world_matrix(world_matrix);
            }

            // Update all children with this object's world matrix
            let world_matrix = *object.transform.get_world_matrix();
            for &child_id in &object.child_ids {
                self.update_object_transform_recursive(child_id, world_matrix);
            }
        }
    }

    /// Invalidates world transforms for an object and all its descendants
    pub fn invalidate_object_hierarchy(&self, object_id: ObjectId) {
        if let Some(object) = self.objects.get(object_id) {
            object.transform.invalidate_world();

            for &child_id in &object.child_ids {
                self.invalidate_object_hierarchy(child_id);
            }
        }
    }

    /// Sets the parent of an object and updates child relationships
    pub fn set_object_parent(&mut self, child_id: ObjectId, new_parent_id: Option<ObjectId>) {
        // Remove from old parent's children list
        if let Some(child) = self.objects.get(child_id) {
            if let Some(old_parent_id) = child.parent_id {
                if let Some(old_parent) = self.objects.get_mut(old_parent_id) {
                    old_parent.child_ids.retain(|&id| id != child_id);
                }
            }
        }

        // Set new parent and add to new parent's children list
        if let Some(child) = self.objects.get_mut(child_id) {
            child.parent_id = new_parent_id;

            if let Some(new_parent_id) = new_parent_id {
                if let Some(new_parent) = self.objects.get_mut(new_parent_id) {
                    new_parent.child_ids.push(child_id);
                }
            }
        }

        // Invalidate world transforms for the moved object and its descendants
        self.invalidate_object_hierarchy(child_id);
    }

    #[allow(dead_code)]
    pub fn set_object_translation(&mut self, object_id: ObjectId, translation: Vec3) {
        if let Some(object) = self.objects.get_mut(object_id) {
            object.transform.set_translation(translation);
        }
        self.invalidate_object_hierarchy(object_id);
    }

    #[allow(dead_code)]
    pub fn set_object_rotation(&mut self, object_id: ObjectId, rotation: Quat) {
        if let Some(object) = self.objects.get_mut(object_id) {
            object.transform.set_rotation(rotation);
        }
        self.invalidate_object_hierarchy(object_id);
    }

    #[allow(dead_code)]
    pub fn set_object_scale(&mut self, object_id: ObjectId, scale: f32) {
        if let Some(object) = self.objects.get_mut(object_id) {
            object.transform.set_scale(scale);
        }
        self.invalidate_object_hierarchy(object_id);
    }

    #[allow(dead_code)]
    pub fn set_object_transform(
        &mut self,
        object_id: ObjectId,
        translation: Vec3,
        rotation: Quat,
        scale: f32,
    ) {
        if let Some(object) = self.objects.get_mut(object_id) {
            object.transform.set_transform(translation, rotation, scale);
        }
        self.invalidate_object_hierarchy(object_id);
    }

    #[allow(dead_code)]
    pub fn get_object_transform(&self, object_id: ObjectId) -> Option<&Transform> {
        self.objects.get(object_id).map(|object| &object.transform)
    }

    pub fn early_update(&mut self) {
        // TODO: fork or replace id-arena to support parallel iteration
        for (_, object) in self.objects.iter() {
            object.transform.reset_flags();
        }
    }

    pub fn late_update(&mut self) {
        self.update_transforms();
    }
}
