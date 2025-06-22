use glam::Vec3;
use id_arena::Id;

use crate::scene_graph::scene::Scene;
use crate::scene_graph::scene_model::SceneModelId;
use crate::scene_graph::transform::Transform;

pub type ObjectId = Id<Object3D>;

pub struct Object3D {
    pub name: String,
    pub transform: Transform,
    pub model_id: Option<SceneModelId>,
    pub parent_id: Option<ObjectId>,
    pub child_ids: Vec<ObjectId>,
}

impl Object3D {
    #[allow(dead_code)]
    pub fn parent<'a>(&self, scene: &'a Scene) -> Option<&'a Object3D> {
        self.parent_id.and_then(|id| scene.get_object(id))
    }

    #[allow(dead_code)]
    pub fn children<'a, 'b>(&'a self, scene: &'b Scene) -> impl Iterator<Item = &'b Object3D> + 'b
    where
        'a: 'b,
    {
        self.child_ids
            .iter()
            .filter_map(move |id| scene.get_object(*id))
    }
}

impl Default for Object3D {
    fn default() -> Self {
        Self {
            name: String::new(),
            transform: Transform::from_translation(Vec3::ZERO),
            model_id: None,
            parent_id: None,
            child_ids: Vec::new(),
        }
    }
}
