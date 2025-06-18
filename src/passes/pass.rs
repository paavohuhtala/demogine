use std::sync::Arc;

use wgpu::RenderPass;

use crate::{
    render_common::RenderCommon,
    shader_loader::{PipelineCache, PipelineCacheBuilder},
};

pub(crate) trait Pass {
    type TextureViews;

    fn create(
        device: &wgpu::Device,
        common: Arc<RenderCommon>,
        cache_builder: &mut PipelineCacheBuilder,
    ) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn render<'a, F>(
        &self,
        texture_views: &Self::TextureViews,
        encoder: &mut wgpu::CommandEncoder,
        pipeline_cache: &PipelineCache,
        render_callback: F,
    ) where
        F: FnOnce(&mut RenderPass) + 'a;
}
