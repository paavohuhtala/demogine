use id_arena::Id;

use crate::{model::Model, rendering::render_model::RenderModelId};

pub type SceneModelId = Id<SceneModel>;

pub struct SceneModel {
    pub name: String,
    pub model: Model,
    pub render_model: Option<RenderModelId>,
}

impl SceneModel {
    pub fn new(name: String, model: Model) -> Self {
        Self {
            name,
            model,
            render_model: None,
        }
    }
}
