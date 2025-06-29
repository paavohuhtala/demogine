use anyhow::Result;

mod asset_pipeline;
mod camera;
mod demo;
mod engine;
mod material_manager;
mod math;
mod model;
mod rendering;
mod scene_graph;
mod window;

fn main() -> Result<()> {
    pretty_env_logger::init();

    pollster::block_on(window::run())?;

    Ok(())
}
