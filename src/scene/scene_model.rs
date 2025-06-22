use id_arena::Id;

use crate::{
    model::{Instance, Model, RenderModelId},
    rendering::instance::Instances,
};

pub type SceneModelId = Id<SceneModel>;

pub struct SceneModel {
    pub name: String,
    pub model: Model,
    pub render_model: Option<RenderModelId>,
    instances: Instances,
}

impl SceneModel {
    pub fn new(name: String, model: Model) -> Self {
        Self {
            name,
            model,
            render_model: None,
            instances: Instances::new(),
        }
    }

    pub fn instances(&self) -> &Instances {
        &self.instances
    }

    pub fn add_instance(&mut self, instance: Instance) {
        self.instances.add(instance);
    }

    pub fn clear_instances(&mut self) {
        self.instances.clear();
    }
}
