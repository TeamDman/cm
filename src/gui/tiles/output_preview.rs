//! Output preview tile - shows renamed files with status colors

use crate::gui::folder_picker;
use crate::gui::state::AppState;
use crate::gui::tree_view::group_files_with_renames_and_options;
use crate::gui::tree_view::open_in_explorer;
use crate::gui::tree_view::show_rename_group_with_output_path;
use crate::image_processing::get_output_dir;
use eframe::egui::Color32;
use eframe::egui::ScrollArea;
use eframe::egui::{self};

/// Draw the output preview tile UI
#[expect(clippy::cast_precision_loss)]
pub fn draw_output_preview_tile(ui: &mut egui::Ui, state: &mut AppState) {
    // Update the rename preview cache if needed
    state.update_rename_preview();

    // Header with Process All and Process Selected buttons
    ui.horizontal(|ui| {
        ui.heading("Output Preview");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Process All button - when running show a Cancel button, otherwise allow starting
            let process_all_btn = egui::Button::new("▶ Process All");
            if state.process_all_running {
                // Cancel button appears while processing
                if ui.add(egui::Button::new("✖ CANCEL")).clicked() {
                    state.cancel_process_all();
                }
                ui.add_enabled(false, process_all_btn);
            } else if ui.add(process_all_btn).clicked() {
                state.process_all();
            }

            // Process Selected button - disable while processing or if nothing selected
            let process_selected_btn = egui::Button::new("▶ Process Selected");
            let can_process_selected =
                !state.process_all_running && state.selected_input_file.is_some();
            if !can_process_selected {
                ui.add_enabled(false, process_selected_btn);
            } else if ui.add(process_selected_btn).clicked() {
                state.process_selected();
            }
        });
    });

    // Show processing progress if running
    if state.process_all_running {
        if let Some((current, total)) = state.process_all_progress {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Processing {current}/{total}..."));
            });
            let progress = current as f32 / total.max(1) as f32;
            ui.add(egui::ProgressBar::new(progress).show_percentage());
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Starting...");
            });
        }
        ui.separator();
    }

    draw_shared_output_folder_controls(ui, state);

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

    let output_path_options = state.output_path_options();
    let grouped = group_files_with_renames_and_options(
        &state.input_paths,
        &state.image_files,
        &state.renamed_files,
        state.max_name_length,
        &output_path_options,
    );
    let shared_output_dir = output_path_options.shared_output_dir.clone();

    ScrollArea::both()
        .id_salt("output_preview_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (input_path, files_info) in &grouped {
                // Show with -output suffix
                let output_dir = shared_output_dir
                    .clone()
                    .unwrap_or_else(|| get_output_dir(input_path));
                let result = show_rename_group_with_output_path(
                    ui,
                    input_path,
                    &output_dir,
                    files_info,
                    state.max_name_length,
                    state.selected_input_file.as_ref(),
                );
                if let Some(clicked) = result.clicked_path {
                    state.select_file(&clicked);
                }
            }
        });
}

fn draw_shared_output_folder_controls(ui: &mut egui::Ui, state: &mut AppState) {
    if !state.save_all_inputs_to_same_folder {
        return;
    }

    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Output folder:");

        let effective_dir = state.effective_shared_output_dir();
        let mut path_text = effective_dir
            .as_ref()
            .map_or_else(String::new, |path| path.display().to_string());

        let text_response = ui.add_sized(
            [ui.available_width().max(240.0) - 160.0, 20.0],
            egui::TextEdit::singleline(&mut path_text),
        );
        if text_response.changed() {
            let trimmed = path_text.trim();
            state.shared_output_dir = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.into())
            };
        }
    });

    ui.horizontal(|ui| {
        if state.shared_output_dir.is_none() {
            ui.label("Using default");
        } else if ui.button("Use Default").clicked() {
            state.shared_output_dir = None;
        }

        if ui.button("Browse").clicked()
            && let Some(path) = folder_picker::pick_folder()
        {
            state.shared_output_dir = Some(path);
        }

        let effective_dir = state.effective_shared_output_dir();
        let can_open = effective_dir.is_some();
        if ui
            .add_enabled(can_open, egui::Button::new("Open"))
            .clicked()
            && let Some(path) = effective_dir
        {
            if let Err(e) = std::fs::create_dir_all(&path) {
                tracing::error!("Failed to create output folder {}: {}", path.display(), e);
            } else {
                open_in_explorer(&path);
            }
        }
    });
    ui.separator();
}
