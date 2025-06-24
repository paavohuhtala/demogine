use crate::{demo::DemoState, rendering::renderer::Renderer};

pub fn update(
    state: &mut DemoState,
    _renderer: &mut Renderer,
    _ui: &mut imgui::Ui,
) -> anyhow::Result<()> {
    state.scene.early_update();
    state.update();
    state.scene.late_update();

    Ok(())
}
