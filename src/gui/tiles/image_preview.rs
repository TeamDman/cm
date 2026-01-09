//! Image preview tile - shows input or output image preview

use crate::gui::state::AppState;
use eframe::egui::{self, load::SizedTexture, ScrollArea, TextureHandle, TextureOptions, Vec2};
use std::path::PathBuf;

/// Draw an image preview tile for input images
pub fn draw_input_image_preview_tile(ui: &mut egui::Ui, state: &mut AppState) {
    let path = state.input_preview_path.clone();
    let should_clear = draw_image_preview(ui, path.as_ref(), "input");
    if should_clear {
        state.selected_input_file = None;
        state.input_preview_path = None;
        state.output_preview_path = None;
        state.selected_output_info = None;
    }
}

/// Draw an image preview tile for output images (shows processed/cropped version)
pub fn draw_output_image_preview_tile(
    ui: &mut egui::Ui,
    state: &mut AppState,
    output_texture: &mut Option<TextureHandle>,
    output_texture_path: &mut Option<PathBuf>,
) {
    // Show output info header
    if let Some(ref output_info) = state.selected_output_info {
        ui.horizontal(|ui| {
            if output_info.was_cropped {
                ui.label(format!(
                    "📐 Cropped: {}x{} → {}x{}",
                    output_info.original_width,
                    output_info.original_height,
                    output_info.output_width,
                    output_info.output_height
                ));
            } else {
                ui.label(format!(
                    "📐 {}x{}",
                    output_info.original_width,
                    output_info.original_height
                ));
            }
        });
        ui.separator();
    } else if state.output_info_loading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Processing...");
        });
        ui.separator();
    }
    
    // Show the processed image preview
    let should_clear = draw_output_preview_with_texture(
        ui,
        state,
        output_texture,
        output_texture_path,
    );
    
    if should_clear {
        state.selected_input_file = None;
        state.input_preview_path = None;
        state.output_preview_path = None;
        state.selected_output_info = None;
        *output_texture = None;
        *output_texture_path = None;
    }
}

/// Draw output preview using the processed image data
fn draw_output_preview_with_texture(
    ui: &mut egui::Ui,
    state: &AppState,
    texture: &mut Option<TextureHandle>,
    texture_path: &mut Option<PathBuf>,
) -> bool {
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
                
                let label = if state.crop_to_content {
                    format!("{} (cropped preview)", filename)
                } else {
                    filename
                };
                
                let response = ui.label(&label);
                response.on_hover_text(input_path.display().to_string());
            });
            ui.separator();
            
            // Update texture if we have new output info
            if let Some(ref output_info) = state.selected_output_info {
                // Check if we need to reload the texture
                let needs_reload = texture_path.as_ref() != Some(input_path) 
                    || texture.is_none();
                
                if needs_reload {
                    // Load the processed image from PNG bytes
                    if let Ok(image) = image::load_from_memory(&output_info.preview_data) {
                        let size = [image.width() as _, image.height() as _];
                        let rgba = image.to_rgba8();
                        let pixels = rgba.as_flat_samples();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_slice(),
                        );
                        
                        *texture = Some(ui.ctx().load_texture(
                            format!("output_preview_{}", input_path.display()),
                            color_image,
                            TextureOptions::default(),
                        ));
                        *texture_path = Some(input_path.clone());
                    }
                }
                
                // Show the texture if we have it
                if let Some(tex) = texture {
                    let available = ui.available_size();
                    let tex_size = tex.size_vec2();
                    
                    // Scale to fit while maintaining aspect ratio (allow shrinking)
                    let scale = (available.x / tex_size.x).min(available.y / tex_size.y);
                    let display_size = Vec2::new(tex_size.x * scale, tex_size.y * scale);
                    
                    ui.centered_and_justified(|ui| {
                        ui.image(SizedTexture::new(tex.id(), display_size));
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
                // Fallback to showing original file
                let uri = format!("file://{}", input_path.display());
                ScrollArea::both()
                    .id_salt("output_image_scroll_fallback")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let available = ui.available_size();
                        let image = egui::Image::new(&uri)
                            .fit_to_exact_size(Vec2::new(available.x, available.y))
                            .maintain_aspect_ratio(true)
                            .shrink_to_fit();
                        ui.add(image);
                    });
            }
        }
        None => {
            // Clear texture when no file is selected
            if texture.is_some() {
                *texture = None;
                *texture_path = None;
            }
            
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label("Click an image in the output tree to preview it here.");
                ui.add_space(10.0);
                ui.label("When crop is enabled, the cropped version will be shown.");
            });
        }
    }

    should_clear
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
            
            let available = ui.available_size();
            
            // Create the image widget - scale to fit available space
            let image = egui::Image::new(&uri)
                .max_size(available)
                .fit_to_original_size(1.0)
                .shrink_to_fit();
            
            ui.add(image);
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
