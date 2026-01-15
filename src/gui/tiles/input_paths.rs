//! Input paths tile - shows and manages input directories

use crate::gui::state::AppState;
use eframe::egui::ScrollArea;
use eframe::egui::{self};

/// Draw the input paths tile UI
pub fn draw_input_paths_tile(ui: &mut egui::Ui, state: &mut AppState) {
    // Clear all button and instructions
    ui.horizontal(|ui| {
        if ui.button("🗑 Clear All").clicked() {
            state.clear_all = true;
        }
        ui.label("Drag & drop folders here");
    });

    ui.separator();

    // Show loading state
    if state.input_paths_loading.is_loading() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading input paths...");
        });
        return;
    }

    if state.input_paths.is_empty() {
        ui.label("(no inputs - drag folders onto the window)");
        return;
    }

    // Show paths with remove buttons - use both scroll areas for long paths
    ScrollArea::both()
        .id_salt("inputs_paths_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for path in &state.input_paths {
                ui.horizontal(|ui| {
                    // Show spinner if image files are still being discovered
                    if state.image_files_loading.is_loading() {
                        ui.spinner();
                    }

                    // Remove button
                    if ui.small_button("✖").clicked() {
                        state.path_to_remove = Some(path.clone());
                    }
                    // Path label - use selectable_label to allow text selection
                    let display = path.display().to_string();
                    ui.add(egui::Label::new(&display).wrap_mode(egui::TextWrapMode::Extend));
                });
            }
        });
}
