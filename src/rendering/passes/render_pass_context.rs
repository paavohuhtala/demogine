use std::sync::Arc;

use crate::rendering::{
    config::RenderConfig,
    instancing::DrawableBuffers,
    mesh_buffers::MeshBuffers,
    render_common::RenderCommon,
    render_material_manager::RenderMaterialManager,
    shader_loader::{PipelineCacheBuilder, RenderPipelineCache},
};

pub struct PassCreationContext {
    pub device: wgpu::Device,
    pub config: &'static RenderConfig,
    pub common: Arc<RenderCommon>,

    pub drawable_buffers: Arc<DrawableBuffers>,
    pub mesh_buffers: Arc<MeshBuffers>,
}

pub struct RenderPassCreationContext<'a> {
    pub shared: &'a PassCreationContext,
    pub cache_builder: &'a mut PipelineCacheBuilder<wgpu::RenderPipeline>,
    pub material_manager: &'a RenderMaterialManager,
    pub camera_uniform_buffer: &'a wgpu::Buffer,
}

pub struct ComputePassCreationContext<'a> {
    pub shared: &'a PassCreationContext,
    pub cache_builder: &'a mut PipelineCacheBuilder<wgpu::ComputePipeline>,
}

pub struct RenderPassContext<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub pipeline_cache: &'a RenderPipelineCache,
    pub draw_commands_buffer: &'a wgpu::Buffer,
    pub draw_commands_count_buffer: &'a wgpu::Buffer,
    pub material_manager: &'a mut RenderMaterialManager,
}
