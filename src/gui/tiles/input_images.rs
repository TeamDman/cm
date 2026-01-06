//! Input images tree tile - shows discovered image files

use crate::gui::state::AppState;
use crate::gui::tree_view::{group_files_by_input, show_input_group};
use eframe::egui::{self, ScrollArea};

/// Draw the input images tree tile UI
pub fn draw_input_images_tile(ui: &mut egui::Ui, state: &mut AppState) {
    if state.image_files.is_empty() {
        ui.label("(no image files found)");
        ui.add_space(8.0);
        ui.label("Add input directories to discover images.");
        return;
    }

    ui.label("Click an image to preview it:");
    ui.separator();

    // Build a tree structure grouped by input directories
    let grouped = group_files_by_input(&state.input_paths, &state.image_files);

    ScrollArea::both()
        .id_salt("images_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (input_path, relative_files) in &grouped {
                let result = show_input_group(
                    ui,
                    input_path,
                    relative_files,
                    state.selected_input_file.as_ref(),
                );
                if let Some(clicked) = result.clicked_path {
                    state.select_file(clicked);
                }
            }
        });
}
