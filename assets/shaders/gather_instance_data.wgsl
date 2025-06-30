
#import shared::drawable::InputDrawable
#import shared::drawable::VisibleDrawable

@group(0) @binding(0)
var<storage, read> drawables: array<InputDrawable>;
@group(0) @binding(1)
var<storage, read> drawable_visibility: array<u32>;
@group(0) @binding(2)
var<storage, read> base_offsets: array<u32>;
@group(0) @binding(3)
var<storage, read_write> visible_drawables: array<VisibleDrawable>;
@group(0) @binding(4)
var<storage, read_write> drawable_local_indices: array<atomic<u32>>;

@compute @workgroup_size(64)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let index = global_id.x;

    if index >= arrayLength(&drawables) {
        return;
    }

    let drawable = drawables[index];
    let visibility = drawable_visibility[index];

    if visibility == 0 {
        return;
    }

    let mesh_index = drawable.mesh_index;
    let base_offset = base_offsets[mesh_index];
    let local_offset = atomicAdd(&drawable_local_indices[mesh_index], 1u);

    visible_drawables[base_offset + local_offset] = VisibleDrawable(
        drawable.model_matrix,
        drawable.inverse_transpose_model_matrix,
        mesh_index,
        drawable.material_id,
        array<u32, 2>(0, 0)
    );
}
