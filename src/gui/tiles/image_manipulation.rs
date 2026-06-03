//! Image manipulation settings tile

use crate::gui::state::AppState;
use crate::image_processing::BinarizationMode;
use crate::image_processing::DEFAULT_MAX_FILE_SIZE_BYTES;
use eframe::egui;
use std::fs;

/// Draw the image manipulation settings tile UI
#[expect(clippy::too_many_lines)]
pub fn draw_image_manipulation_tile(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Image Manipulation");
    ui.separator();

    let manipulation_changed = ui
        .checkbox(&mut state.image_manipulation_enabled, "Modify image data")
        .on_hover_text("Turn off to preserve original image bytes and only apply filename changes")
        .changed();

    ui.add_space(8.0);

    ui.add_enabled_ui(state.image_manipulation_enabled, |ui| {
        // Crop to content checkbox
        let mut crop_changed = ui
            .checkbox(&mut state.crop_to_content, "Crop images to content")
            .on_hover_text("Remove padding from images based on threshold")
            .changed();

        ui.add_space(8.0);

        ui.add_enabled_ui(state.crop_to_content, |ui| {
            ui.horizontal(|ui| {
                ui.label("Threshold:");
                let threshold_changed = ui
                    .add(egui::Slider::new(&mut state.crop_threshold, 0..=255).text("tolerance"))
                    .changed();

                if threshold_changed {
                    crop_changed = true;
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Preview mode:");
                let mode_changed = egui::ComboBox::from_id_salt("binarization_mode")
                    .selected_text(match state.binarization_mode {
                        BinarizationMode::KeepWhite => "Keep White",
                        BinarizationMode::KeepBlack => "Keep Black",
                    })
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        changed |= ui
                            .selectable_value(
                                &mut state.binarization_mode,
                                BinarizationMode::KeepWhite,
                                "Keep White",
                            )
                            .on_hover_text("Show content as black, background as white")
                            .clicked();
                        changed |= ui
                            .selectable_value(
                                &mut state.binarization_mode,
                                BinarizationMode::KeepBlack,
                                "Keep Black",
                            )
                            .on_hover_text("Show content as white, background as black")
                            .clicked();
                        changed
                    })
                    .inner
                    .unwrap_or(false);

                if mode_changed {
                    crop_changed = true;
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let box_visibility_changed = ui
                    .checkbox(&mut state.show_crop_bounding_box, "Show crop preview box")
                    .changed();

                if box_visibility_changed {
                    crop_changed = true;
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Box thickness:");
                let thickness_changed = ui
                    .add_enabled(
                        state.show_crop_bounding_box,
                        egui::Slider::new(&mut state.box_thickness, 1..=50).text("px"),
                    )
                    .changed();

                if thickness_changed {
                    crop_changed = true;
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // JPEG quality slider
        ui.horizontal(|ui| {
            ui.label("JPEG quality:");
            let quality_changed = ui
                .add(egui::Slider::new(&mut state.jpeg_quality, 1..=100).text("%"))
                .changed();

            if quality_changed && state.selected_input_file.is_some() {
                state.update_selected_output_info();
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let mut max_size_enabled = state.max_file_size_bytes.is_some();
            let enabled_changed = ui
                .checkbox(&mut max_size_enabled, "Reduce file size")
                .on_hover_text("Shrink output images when needed to fit this size")
                .changed();

            if enabled_changed {
                state.max_file_size_bytes = max_size_enabled.then_some(DEFAULT_MAX_FILE_SIZE_BYTES);
            }

            let mut max_size_kb = state
                .max_file_size_bytes
                .unwrap_or(DEFAULT_MAX_FILE_SIZE_BYTES)
                .div_ceil(1024);
            let size_changed = ui
                .add_enabled(
                    max_size_enabled,
                    egui::DragValue::new(&mut max_size_kb)
                        .range(1..=10 * 1024 * 1024)
                        .speed(10)
                        .suffix(" KB"),
                )
                .changed();

            if max_size_enabled && size_changed {
                state.max_file_size_bytes = Some(max_size_kb.saturating_mul(1024));
            }

            if (enabled_changed || size_changed) && state.selected_input_file.is_some() {
                state.update_selected_output_info();
            }
        });

        ui.add_space(4.0);

        // Sync pan/zoom checkbox
        ui.checkbox(&mut state.sync_preview_pan_zoom, "Sync preview pan/zoom")
            .on_hover_text("Synchronize pan and zoom across input, threshold, and output previews");

        // Recalculate output info if settings changed
        if (crop_changed || manipulation_changed) && state.selected_input_file.is_some() {
            state.update_selected_output_info();
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    ui.checkbox(
        &mut state.flatten_output_hierarchy,
        "Flatten output hierarchy",
    )
    .on_hover_text(
        "Save files directly under the output folder and add numbers for name conflicts",
    );

    ui.checkbox(
        &mut state.save_all_inputs_to_same_folder,
        "Save all inputs to the same output folder",
    )
    .on_hover_text("Use one shared output folder when multiple input paths are added");

    ui.add_space(8.0);

    // Show filesize of selected image
    if let Some(ref input_path) = state.selected_input_file {
        ui.separator();
        ui.label("Selected image info:");

        // Show input filesize
        match fs::metadata(input_path) {
            Ok(meta) => {
                let size = meta.len();
                ui.horizontal(|ui| {
                    ui.label("Input size:");
                    ui.strong(format_size(size));
                });
            }
            Err(_) => {
                ui.colored_label(egui::Color32::RED, "Could not read file");
            }
        }

        // Show loading state or output info
        if state.output_info_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Processing...");
            });
        } else if let Some(ref output_info) = state.selected_output_info {
            ui.horizontal(|ui| {
                ui.label("Output size:");
                ui.strong(format_size(output_info.estimated_size));
            });

            if output_info.was_cropped {
                ui.horizontal(|ui| {
                    ui.label("Dimensions:");
                    ui.label(format!(
                        "{}x{} => {}x{}",
                        output_info.original_width,
                        output_info.original_height,
                        output_info.output_width,
                        output_info.output_height
                    ));
                });
            }
        }
    } else {
        ui.add_space(8.0);
        ui.label("Select an image to see size info");
    }
}

/// Format file size in human-readable form
#[expect(clippy::cast_precision_loss)]
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} bytes")
    }
}
