use crate::rendering::{
    mesh_buffers::MeshBuffers, render_material_manager::RenderMaterialManager,
    shader_loader::RenderPipelineCache,
};

pub struct RenderPassContext<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub pipeline_cache: &'a RenderPipelineCache,
    pub instance_bind_group: &'a wgpu::BindGroup,
    pub indirect_buffer: &'a wgpu::Buffer,
    pub mesh_buffers: &'a MeshBuffers,
    pub material_manager: &'a mut RenderMaterialManager,
}
