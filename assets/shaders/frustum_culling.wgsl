#import shared::drawable::Drawable
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
var<storage, read> primitives: array<MeshInfo>;
@group(0) @binding(2)
var<storage, read> drawables: array<Drawable>;
@group(0) @binding(3)
var<storage, read_write> draw_commands: array<DrawIndexedIndirectCommand>;

@compute @workgroup_size(64)
fn cull(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let index = global_id.x;

    if index >= arrayLength(&drawables) {
        return;
    }

    let drawable = drawables[index];

    let primitive_index = drawable.primitive_index;
    let primitive = primitives[primitive_index];
    let aabb = AABB(primitive.aabb_min, primitive.aabb_max);
    
        // Perform actual frustum culling
    if is_inside_frustum_transformed(aabb, drawable.model_matrix, frustum) {
        draw_commands[index] = DrawIndexedIndirectCommand(
            primitive.index_count,
            1,
            primitive.first_index,
            primitive.vertex_offset,
            index
        );
    } else {
        draw_commands[index] = DrawIndexedIndirectCommand(
            0, // index_count
            0, // instance_count
            0, // first_index
            0, // vertex_offset
            0  // first_instance
        );
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
