//! Input images tree tile - shows discovered image files

use crate::gui::state::AppState;
use crate::gui::tree_view::{group_files_by_input, show_input_group_with_cache, TreeRenderContext};
use eframe::egui::{self, ScrollArea, TextureHandle};
use std::collections::HashMap;
use std::path::PathBuf;

/// Draw the input images tree tile UI
pub fn draw_input_images_tile(
    ui: &mut egui::Ui,
    state: &mut AppState,
    thumbnail_textures: &mut HashMap<PathBuf, TextureHandle>,
) {
    if state.image_files.is_empty() {
        ui.label("(no image files found)");
        ui.add_space(8.0);
        ui.label("Add input directories to discover images.");
        return;
    }

    // Show loading progress if images are still being cached
    let loading_count = state.images_loading.len();
    let cached_count = state.image_cache.len();
    let total_count = state.image_files.len();
    
    if loading_count > 0 {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(format!("Loading images... ({}/{} cached)", cached_count, total_count));
        });
    } else {
        ui.label(format!("Click an image to preview it ({} images):", total_count));
    }
    ui.separator();

    // Build a tree structure grouped by input directories
    let grouped = group_files_by_input(&state.input_paths, &state.image_files);

    let mut clicked_file: Option<PathBuf> = None;
    
    ScrollArea::both()
        .id_salt("images_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut ctx = TreeRenderContext {
                image_cache: &state.image_cache,
                images_loading: &state.images_loading,
                thumbnail_textures,
            };
            
            for (input_path, relative_files) in &grouped {
                let result = show_input_group_with_cache(
                    ui,
                    input_path,
                    relative_files,
                    state.selected_input_file.as_ref(),
                    Some(&mut ctx),
                );
                if result.clicked_path.is_some() {
                    clicked_file = result.clicked_path;
                }
            }
        });
    
    // Handle clicked file after the borrow ends
    if let Some(clicked) = clicked_file {
        state.select_file(clicked);
    }
}
