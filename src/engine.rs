use crate::{demo::DemoState, material_manager::MaterialManager, rendering::renderer::Renderer};

pub fn update(
    state: &mut DemoState,
    _renderer: &mut Renderer,
    material_manager: &mut MaterialManager,
    ui: &mut imgui::Ui,
) -> anyhow::Result<()> {
    state.scene.early_update();
    state.update();
    state.scene.late_update();

    material_manager.draw_ui(ui);

    Ok(())
}
