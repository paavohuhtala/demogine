use glam::{Mat4, Vec3};

use crate::math::{frustum::Frustum, plane::Plane};

pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    fn signed_distance_to_plane(&self, plane: &Plane) -> f32 {
        plane.signed_distance_to_point(self.center) - self.radius
    }

    pub fn transform(&self, matrix: &Mat4) -> BoundingSphere {
        let center = matrix.transform_point3(self.center);
        let scale = matrix.to_scale_rotation_translation().0;
        let radius = self.radius * scale.max_element();
        BoundingSphere { center, radius }
    }

    pub fn intersects_frustum(&self, frustum: &Frustum) -> bool {
        for plane in &frustum.planes {
            if self.signed_distance_to_plane(plane) < 0.0 {
                return false;
            }
        }

        true
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }
}

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

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn corners(&self) -> [Vec3; 8] {
        [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ]
    }

    pub fn intersects_frustum_transformed(&self, frustum: &Frustum, transform: &Mat4) -> bool {
        let corners = self
            .corners()
            .map(|corner| transform.transform_point3(corner));

        for plane in &frustum.planes {
            let mut outside = true;

            for corner in &corners {
                if plane.signed_distance_to_point(*corner) >= 0.0 {
                    outside = false;
                    break;
                }
            }

            if outside {
                return false;
            }
        }

        true
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }
}
