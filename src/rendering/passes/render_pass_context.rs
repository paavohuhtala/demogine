use crate::rendering::{
    mesh_buffers::MeshBuffers, render_material_manager::RenderMaterialManager,
    shader_loader::RenderPipelineCache,
};

pub struct RenderPassContext<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub pipeline_cache: &'a RenderPipelineCache,
    pub drawable_bind_group: &'a wgpu::BindGroup,
    pub draw_commands_buffer: &'a wgpu::Buffer,
    pub draw_commands_count_buffer: &'a wgpu::Buffer,
    pub mesh_buffers: &'a MeshBuffers,
    pub material_manager: &'a mut RenderMaterialManager,
}
