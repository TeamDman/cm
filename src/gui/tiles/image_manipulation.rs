//! Image manipulation settings tile

use crate::gui::state::AppState;
use eframe::egui;
use std::fs;

/// Draw the image manipulation settings tile UI
pub fn draw_image_manipulation_tile(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Image Manipulation");
    ui.separator();

    // Crop to content checkbox
    let crop_changed = ui.checkbox(&mut state.crop_to_content, "Crop images to content")
        .on_hover_text("Remove white/transparent padding from images")
        .changed();
    
    // Recalculate output info if crop setting changed
    if crop_changed && state.selected_input_file.is_some() {
        state.update_selected_output_info();
    }

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

        // If we have a cached output, show its info
        if let Some(ref output_info) = state.selected_output_info {
            ui.horizontal(|ui| {
                ui.label("Output size:");
                ui.strong(format_size(output_info.estimated_size));
            });
            
            if output_info.was_cropped {
                ui.horizontal(|ui| {
                    ui.label("Dimensions:");
                    ui.label(format!(
                        "{}x{} → {}x{}",
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
        format!("{} bytes", bytes)
    }
}
