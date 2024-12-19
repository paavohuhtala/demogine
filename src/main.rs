use anyhow::Result;

mod camera;
mod model;
mod texture;
mod window;

fn main() -> Result<()> {
    pretty_env_logger::init();

    pollster::block_on(window::run())?;

    Ok(())
}
