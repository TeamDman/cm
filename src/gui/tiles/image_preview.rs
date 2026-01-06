//! Image preview tile - shows input or output image preview

use crate::gui::state::AppState;
use eframe::egui;
use std::path::PathBuf;

/// Draw an image preview tile for input images
pub fn draw_input_image_preview_tile(ui: &mut egui::Ui, state: &AppState) {
    draw_image_preview(ui, state.input_preview_path.as_ref(), "input");
}

/// Draw an image preview tile for output images
pub fn draw_output_image_preview_tile(ui: &mut egui::Ui, state: &AppState) {
    draw_image_preview(ui, state.output_preview_path.as_ref(), "output");
}

fn draw_image_preview(ui: &mut egui::Ui, path: Option<&PathBuf>, kind: &str) {
    match path {
        Some(path) => {
            ui.label(format!("Previewing: {}", path.display()));
            ui.separator();

            // TODO: Actually load and display the image
            // For now, just show the path
            ui.centered_and_justified(|ui| {
                ui.label(format!("(Image preview for {} not yet implemented)", kind));
            });
        }
        None => {
            ui.centered_and_justified(|ui| {
                ui.label(format!("Click a file in the tree view to preview it as {}", kind));
            });
        }
    }
}
