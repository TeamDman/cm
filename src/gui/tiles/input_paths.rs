//! Input paths tile - shows and manages input directories

use crate::gui::state::AppState;
use eframe::egui::{self, Color32, ScrollArea};

/// Draw the input paths tile UI
pub fn draw_input_paths_tile(ui: &mut egui::Ui, state: &mut AppState) {
    // Show error if any
    if let Some(err) = state.last_error.as_ref() {
        ui.colored_label(Color32::RED, err);
        ui.separator();
    }

    // Clear all button and instructions
    ui.horizontal(|ui| {
        if ui.button("🗑 Clear All").clicked() {
            state.clear_all = true;
        }
        ui.label("Drag & drop folders here");
    });

    ui.separator();

    if state.input_paths.is_empty() {
        ui.label("(no inputs - drag folders onto the window)");
        return;
    }

    // Show paths with remove buttons
    ScrollArea::vertical()
        .id_salt("inputs_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for path in state.input_paths.iter() {
                ui.horizontal(|ui| {
                    // Remove button
                    if ui.small_button("✖").clicked() {
                        state.path_to_remove = Some(path.clone());
                    }
                    // Path label
                    let display = path.display().to_string();
                    ui.label(&display);
                });
            }
        });
}
