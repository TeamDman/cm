//! Image preview tile - shows input or output image preview

use crate::gui::state::AppState;
use eframe::egui::{self, ScrollArea, Vec2};
use std::path::PathBuf;

/// Draw an image preview tile for input images
pub fn draw_input_image_preview_tile(ui: &mut egui::Ui, state: &mut AppState) {
    let path = state.input_preview_path.clone();
    let should_clear = draw_image_preview(ui, path.as_ref(), "input");
    if should_clear {
        state.input_preview_path = None;
    }
}

/// Draw an image preview tile for output images
pub fn draw_output_image_preview_tile(ui: &mut egui::Ui, state: &mut AppState) {
    let path = state.output_preview_path.clone();
    let should_clear = draw_image_preview(ui, path.as_ref(), "output");
    if should_clear {
        state.output_preview_path = None;
    }
}

/// Returns true if the preview should be cleared
fn draw_image_preview(
    ui: &mut egui::Ui,
    path: Option<&PathBuf>,
    kind: &str,
) -> bool {
    let mut should_clear = false;

    match path {
        Some(path) => {
            // Header with path and clear button
            ui.horizontal(|ui| {
                if ui.small_button("✖").clicked() {
                    should_clear = true;
                }
                
                // Show just the filename, with full path on hover
                let filename = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                
                let response = ui.label(&filename);
                response.on_hover_text(path.display().to_string());
            });
            ui.separator();

            // Check if file exists
            if !path.exists() {
                ui.colored_label(egui::Color32::RED, "File not found!");
                ui.label(path.display().to_string());
                return should_clear;
            }

            // Display the image using egui's Image widget with file:// URI
            let uri = format!("file://{}", path.display());
            
            ScrollArea::both()
                .id_salt(format!("{}_image_scroll", kind))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let available = ui.available_size();
                    
                    // Create the image widget - it will auto-size to fit available space
                    let image = egui::Image::new(&uri)
                        .fit_to_exact_size(Vec2::new(available.x, available.y))
                        .maintain_aspect_ratio(true)
                        .shrink_to_fit();
                    
                    ui.add(image);
                });
        }
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(format!("Click an image in the {} tree to preview it here.", kind));
                ui.add_space(10.0);
                ui.label("Images will be displayed at their original resolution.");
            });
        }
    }

    should_clear
}
