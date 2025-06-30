#import shared::drawable::InputDrawable
#import shared::mesh_info::MeshInfo
#import shared::frustum::Frustum
#import shared::commands::DrawIndexedIndirectCommand

struct AABB {
    // W coordinates are unused, but required for alignment
    min: vec4<f32>,
    max: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> frustum: Frustum;
@group(0) @binding(1)
var<storage, read> meshes: array<MeshInfo>;
@group(0) @binding(2)
var<storage, read> drawables: array<InputDrawable>;

@group(0) @binding(3)
var<storage, read_write> drawable_visibility: array<u32>;
@group(0) @binding(4)
var<storage, read_write> visible_drawables_by_mesh: array<atomic<u32>>;

@compute @workgroup_size(64)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let index = global_id.x;

    if index >= arrayLength(&drawables) {
        return;
    }

    let drawable = drawables[index];

    let mesh_index = drawable.mesh_index;
    let mesh = meshes[mesh_index];
    let aabb = AABB(mesh.aabb_min, mesh.aabb_max);

    if is_inside_frustum_transformed(aabb, drawable.model_matrix, frustum) {
        drawable_visibility[index] = 1;
        atomicAdd(&visible_drawables_by_mesh[mesh_index], 1u);
    } else {
        drawable_visibility[index] = 0;
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

            if distance <= 0.0 {
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
