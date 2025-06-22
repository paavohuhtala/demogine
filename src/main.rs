use anyhow::Result;

mod camera;
mod demo;
mod global_uniform;
mod model;
mod passes;
mod render_common;
mod rendering;
mod scene;
mod shader_loader;
mod texture;
mod window;

fn main() -> Result<()> {
    pretty_env_logger::init();

    pollster::block_on(window::run())?;

    Ok(())
}
