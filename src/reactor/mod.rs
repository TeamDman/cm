pub mod plan;

/// Launch the v2 surface.
///
/// # Errors
/// Returns an error if the GUI runtime cannot be created or the surface fails to run.
pub fn run_studio_v2() -> eyre::Result<()> {
    crate::egui::run_gui_with_initial_tool(Some(crate::egui::ToolChoice::V2))
}
