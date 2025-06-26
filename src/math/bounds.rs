use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(point1: Vec3, point2: Vec3) -> AABB {
        let min = point1.min(point2);
        let max = point1.max(point2);
        AABB { min, max }
    }
}
