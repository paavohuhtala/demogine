use anyhow::Result;

mod camera;
mod model;
mod shader_loader;
mod texture;
mod window;

fn main() -> Result<()> {
    pretty_env_logger::init();

    pollster::block_on(window::run())?;

    Ok(())
}
