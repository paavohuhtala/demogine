
#import shared::commands::DrawIndexedIndirectCommand
#import shared::mesh_info::MeshInfo

@group(0) @binding(0)
var<storage, read> mesh_infos: array<MeshInfo>;
@group(0) @binding(1)
var<storage, read> visible_drawables_by_mesh: array<u32>;

@group(0) @binding(2)
var<storage, read_write> base_offsets: array<u32>;
@group(0) @binding(3)
var<storage, read_write> draw_commands: array<DrawIndexedIndirectCommand>;
@group(0) @binding(4)
var<storage, read_write> draw_commands_count: u32;

@compute @workgroup_size(1)
fn main() {
    // Generate base offsets to instance data for each mesh type
    base_offsets[0] = 0;

    for (var i = 1u; i < arrayLength(&visible_drawables_by_mesh) && i < arrayLength(&base_offsets); i++) {
        base_offsets[i] = base_offsets[i - 1] + visible_drawables_by_mesh[i - 1];
    }

    // Generate draw commands for each mesh type
    draw_commands_count = 0;

    for (var i = 0u; i < arrayLength(&visible_drawables_by_mesh); i++) {
        let count = visible_drawables_by_mesh[i];

        // No visible instances for this mesh type, skip
        if count == 0 {
            continue;
        }

        let mesh = mesh_infos[i];
        let base_offset = base_offsets[i];

        draw_commands[draw_commands_count] = DrawIndexedIndirectCommand(
            mesh.index_count,
            count,
            mesh.first_index,
            mesh.vertex_offset,
            base_offset
        );

        draw_commands_count += 1;
    }
}
