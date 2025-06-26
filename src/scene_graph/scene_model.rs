use id_arena::Id;

use crate::model::Model;

pub type SceneModelId = Id<SceneModel>;

pub struct SceneModel {
    pub model: Model,
}

impl SceneModel {
    pub fn new(model: Model) -> Self {
        Self { model }
    }
}
