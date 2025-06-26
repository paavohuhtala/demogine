use bytemuck::{Pod, Zeroable};
use glam::{vec4, Mat4, Vec3, Vec4Swizzles};

use crate::math::plane::Plane;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Frustum {
    // Planes are in the order: left, right, bottom, top, near, far
    pub planes: [Plane; 6],
}

impl Frustum {
    fn corners(view_projection: Mat4) -> [Vec3; 8] {
        let corners: [glam::Vec4; 8] = [
            // Left - Bottom - Near
            vec4(-1.0, -1.0, -1.0, 1.0),
            // Right - Bottom - Near
            vec4(1.0, -1.0, -1.0, 1.0),
            // Left - Top - Near
            vec4(-1.0, 1.0, -1.0, 1.0),
            // Right - Top - Near
            vec4(1.0, 1.0, -1.0, 1.0),
            // Left - Bottom - Far
            vec4(-1.0, -1.0, 1.0, 1.0),
            // Right - Bottom - Far
            vec4(1.0, -1.0, 1.0, 1.0),
            // Left - Top - Far
            vec4(-1.0, 1.0, 1.0, 1.0),
            // Right - Top - Far
            vec4(1.0, 1.0, 1.0, 1.0),
        ];

        let inverse = view_projection.inverse();

        corners.map(|corner| {
            let mut corner = inverse * corner;
            corner = corner / corner.w;
            corner.xyz()
        })
    }

    pub fn from_view_projection(view_projection: Mat4) -> Frustum {
        let corners = Self::corners(view_projection);
        let [left_bottom_near, right_bottom_near, left_top_near, right_top_near, left_bottom_far, right_bottom_far, left_top_far, _right_top_far] =
            corners;

        let planes = [
            // Left
            Plane::from_points(left_bottom_near, left_top_far, left_bottom_far).flip(),
            // Right
            Plane::from_points(right_bottom_near, right_bottom_far, right_top_near).flip(),
            // Bottom
            Plane::from_points(left_bottom_near, right_bottom_near, left_bottom_far),
            // Top
            Plane::from_points(left_top_near, right_top_near, left_top_far).flip(),
            // Near
            Plane::from_points(left_bottom_near, right_bottom_near, left_top_near).flip(),
            // Far
            Plane::from_points(left_bottom_far, right_bottom_far, left_top_far),
        ];

        Frustum { planes }
    }
}
