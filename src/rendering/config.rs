#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub use_multi_draw_indirect_count: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            use_multi_draw_indirect_count: false,
        }
    }
}
