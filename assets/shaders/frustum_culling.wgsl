
struct AABB {
    // W coordinates are unused, but required for alignment
    min: vec4<f32>,
    max: vec4<f32>,
}

struct Cullable {
    aabb: AABB,
    world: mat4x4<f32>,
}

struct Frustum {
    // Same order as in Rust version of the struct
    planes: array<vec4<f32>, 6>,
}

@group(0) @binding(0)
var<uniform> frustum: Frustum;
@group(0) @binding(1)
var<storage, read> cullables: array<Cullable>;
@group(0) @binding(2)
var<storage, read_write> culled_indices: array<u32>;

@compute @workgroup_size(64)
fn cull(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let index = global_id.x;

    if index >= arrayLength(&cullables) {
        return;
    }

    let cullable = cullables[index];
    
    // Transform AABB corners to world space and test against frustum
    if is_inside_frustum_transformed(cullable.aabb, cullable.world, frustum) {
        culled_indices[index] = 1u;
    } else {
        culled_indices[index] = 0u;
    }
}

fn get_aabb_corners(aabb: AABB) -> array<vec3<f32>, 8> {
    return array<vec3<f32>, 8>(
        vec3<f32>(aabb.min.x, aabb.min.y, aabb.min.z),
        vec3<f32>(aabb.max.x, aabb.min.y, aabb.min.z),
        vec3<f32>(aabb.min.x, aabb.max.y, aabb.min.z),
        vec3<f32>(aabb.max.x, aabb.max.y, aabb.min.z),
        vec3<f32>(aabb.min.x, aabb.min.y, aabb.max.z),
        vec3<f32>(aabb.max.x, aabb.min.y, aabb.max.z),
        vec3<f32>(aabb.min.x, aabb.max.y, aabb.max.z),
        vec3<f32>(aabb.max.x, aabb.max.y, aabb.max.z)
    );
}

fn is_inside_frustum_transformed(aabb: AABB, transform: mat4x4<f32>, frustum: Frustum) -> bool {
    let corners = get_aabb_corners(aabb);

    for (var plane_idx = 0; plane_idx < 6; plane_idx++) {
        let plane = frustum.planes[plane_idx];
        var outside = true;

        for (var corner_idx = 0; corner_idx < 8; corner_idx++) {
            let transformed_corner = (transform * vec4<f32>(corners[corner_idx], 1.0)).xyz;
            let distance = dot(plane.xyz, transformed_corner) + plane.w;

            if distance >= 0.0 {
                outside = false;
                break;
            }
        }

        if outside {
            return false;
        }
    }

    return true;
}
