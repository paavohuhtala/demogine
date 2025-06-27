#define_import_path shared::commands

struct DrawIndexedIndirectCommand {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    vertex_offset: u32,
    first_instance: u32,
}
