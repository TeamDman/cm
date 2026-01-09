//! Threshold preview tile - shows binarized image with crop bounding box

use crate::gui::state::AppState;
use crate::gui::tiles::pan_zoom::{PanZoomState, draw_pan_zoom_image};
use eframe::egui::{self, TextureHandle, TextureOptions};
use std::path::PathBuf;

/// Draw the threshold preview tile
pub fn draw_threshold_preview_tile(
    ui: &mut egui::Ui,
    state: &mut AppState,
    threshold_texture: &mut Option<TextureHandle>,
    threshold_texture_path: &mut Option<PathBuf>,
    pan_zoom: &mut PanZoomState,
) {
    let mut should_clear = false;
    
    let current_input = state.selected_input_file.as_ref();
    
    match current_input {
        Some(input_path) => {
            // Header with path and clear button
            ui.horizontal(|ui| {
                if ui.small_button("✖").clicked() {
                    should_clear = true;
                }
                
                let filename = input_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| input_path.display().to_string());
                
                let response = ui.label(format!("{} (threshold)", filename));
                response.on_hover_text(input_path.display().to_string());
            });
            ui.separator();
            
            // Show the threshold preview if we have output info
            if let Some(ref output_info) = state.selected_output_info {
                // Always reload the texture since we need to regenerate when settings change
                let needs_reload = threshold_texture_path.as_ref() != Some(input_path) 
                    || threshold_texture.is_none() 
                    || state.output_info_loading;
                
                if needs_reload {
                    // Load the threshold preview from PNG bytes
                    if let Ok(image) = image::load_from_memory(&output_info.threshold_preview_data) {
                        let size = [image.width() as _, image.height() as _];
                        let rgba = image.to_rgba8();
                        let pixels = rgba.as_flat_samples();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_slice(),
                        );
                        
                        *threshold_texture = Some(ui.ctx().load_texture(
                            format!("threshold_preview_{}", input_path.display()),
                            color_image,
                            TextureOptions::default(),
                        ));
                        *threshold_texture_path = Some(input_path.clone());
                        pan_zoom.reset(); // Reset pan/zoom when loading new image
                    }
                }
                
                // Show the texture with pan/zoom support
                if let Some(tex) = threshold_texture {
                    draw_pan_zoom_image(ui, tex, pan_zoom, "threshold_preview");
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label("Failed to load threshold preview");
                    });
                }
            } else if state.output_info_loading {
                // Show loading spinner
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.spinner();
                    ui.label("Processing image...");
                });
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label("No threshold preview available");
                });
            }
        }
        None => {
            // Clear texture when no file is selected
            if threshold_texture.is_some() {
                *threshold_texture = None;
                *threshold_texture_path = None;
            }
            
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label("Click an image to see threshold preview.");
                ui.add_space(10.0);
                ui.label("Scroll to zoom, drag to pan, double-click to reset.");
            });
        }
    }
    
    if should_clear {
        state.selected_input_file = None;
        state.input_preview_path = None;
        state.output_preview_path = None;
        state.selected_output_info = None;
        *threshold_texture = None;
        *threshold_texture_path = None;
        pan_zoom.reset();
    }
}
