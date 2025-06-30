use glam::{Mat4, Quat, Vec3};
use std::cell::{Cell, Ref, RefCell};

#[derive(Debug, Clone)]
pub struct Transform {
    translation: Vec3,
    rotation: Quat,
    scale: f32,

    local_matrix: RefCell<glam::Mat4>,
    world_matrix: RefCell<glam::Mat4>,
    inverse_transpose_world_matrix: RefCell<glam::Mat4>,
    local_dirty: Cell<bool>,
    world_dirty: Cell<bool>,
    has_changed_since_last_update: Cell<bool>,
}

impl Transform {
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
            scale: 1.0,
            local_matrix: RefCell::new(Mat4::IDENTITY),
            world_matrix: RefCell::new(Mat4::IDENTITY),
            inverse_transpose_world_matrix: RefCell::new(Mat4::IDENTITY),
            local_dirty: Cell::new(true),
            world_dirty: Cell::new(true),
            has_changed_since_last_update: Cell::new(true),
        }
    }

    pub fn get_local_matrix(&self) -> Ref<glam::Mat4> {
        if self.local_dirty.get() {
            let matrix = glam::Mat4::from_scale_rotation_translation(
                Vec3::splat(self.scale),
                self.rotation,
                self.translation,
            );

            self.local_matrix.replace(matrix);
            self.local_dirty.set(false);
            // Not sure about this one - invalidate_local also sets world_dirty
            self.invalidate_world();
        }

        self.local_matrix.borrow()
    }

    pub fn get_world_matrix(&self) -> Ref<glam::Mat4> {
        self.world_matrix.borrow()
    }

    pub fn get_inverse_transpose_world_matrix(&self) -> Ref<glam::Mat4> {
        self.inverse_transpose_world_matrix.borrow()
    }

    pub fn set_world_matrix(&self, world_matrix: Mat4) {
        self.world_matrix.replace(world_matrix);
        self.world_dirty.set(false);
        self.has_changed_since_last_update.set(true);
        self.inverse_transpose_world_matrix
            .replace(world_matrix.inverse().transpose());
    }

    pub fn invalidate_local(&self) {
        self.local_dirty.set(true);
        self.world_dirty.set(true);
        self.has_changed_since_last_update.set(true);
    }

    pub fn invalidate_world(&self) {
        self.world_dirty.set(true);
    }

    pub fn is_world_dirty(&self) -> bool {
        self.world_dirty.get()
    }

    pub fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
        self.invalidate_local();
    }

    #[allow(dead_code)]
    pub fn set_translation(&mut self, translation: Vec3) {
        self.translation = translation;
        self.invalidate_local();
    }

    #[allow(dead_code)]
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
        self.invalidate_local();
    }

    #[allow(dead_code)]
    pub fn translate(&mut self, delta: Vec3) {
        self.translation += delta;
        self.invalidate_local();
    }

    #[allow(dead_code)]
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = self.rotation * rotation;
        self.invalidate_local();
    }

    pub fn set_transform(&mut self, translation: Vec3, rotation: Quat, scale: f32) {
        self.translation = translation;
        self.rotation = rotation;
        self.scale = scale;
        self.invalidate_local();
    }

    #[allow(dead_code)]
    pub fn translation(&self) -> Vec3 {
        self.translation
    }

    #[allow(dead_code)]
    pub fn rotation(&self) -> Quat {
        self.rotation
    }

    #[allow(dead_code)]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn reset_flags(&self) {
        self.has_changed_since_last_update.set(false);
    }

    #[allow(dead_code)]
    pub fn has_changed(&self) -> bool {
        self.has_changed_since_last_update.get()
    }
}
