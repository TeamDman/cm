//! Output preview tile - shows renamed files with status colors

use crate::gui::state::AppState;
use crate::gui::tree_view::{group_files_with_renames, show_rename_group_with_output_path};
use crate::image_processing::get_output_dir;
use eframe::egui::{self, Color32, ScrollArea};

/// Draw the output preview tile UI
pub fn draw_output_preview_tile(ui: &mut egui::Ui, state: &mut AppState) {
    // Update the rename preview cache if needed
    state.update_rename_preview();

    // Header with Process All button
    ui.horizontal(|ui| {
        ui.heading("Output Preview");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("▶ Process All").clicked() {
                state.process_all();
            }
        });
    });

    // Show processing result if any
    if let Some(ref result) = state.processing_result {
        ui.colored_label(
            if result.contains("error") || result.contains("failed") {
                Color32::RED
            } else {
                Color32::LIGHT_GREEN
            },
            result,
        );
        if ui.small_button("✖ dismiss").clicked() {
            state.processing_result = None;
        }
        ui.separator();
    }

    if state.image_files.is_empty() {
        ui.label("(no image files to preview)");
        return;
    }

    // Legend
    ui.horizontal(|ui| {
        ui.colored_label(Color32::LIGHT_GREEN, "●");
        ui.label("unchanged");
        ui.add_space(8.0);
        ui.colored_label(Color32::from_rgb(0xFF, 0xA5, 0x00), "●");
        ui.label("renamed");
        ui.add_space(8.0);
        ui.colored_label(Color32::RED, "●");
        ui.label("too long");
    });

    ui.label("Click an image to preview:");
    ui.separator();

    let grouped = group_files_with_renames(
        &state.input_paths,
        &state.image_files,
        &state.renamed_files,
        state.max_name_length,
    );

    ScrollArea::both()
        .id_salt("output_preview_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (input_path, files_info) in &grouped {
                // Show with -output suffix
                let output_dir = get_output_dir(input_path);
                let result = show_rename_group_with_output_path(
                    ui,
                    input_path,
                    &output_dir,
                    files_info,
                    state.max_name_length,
                    state.selected_input_file.as_ref(),
                );
                if let Some(clicked) = result.clicked_path {
                    state.select_file(clicked);
                }
            }
        });
}
