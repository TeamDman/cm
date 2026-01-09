//! CM GUI using egui_tiles for layout management

mod behavior;
pub mod state;
mod tiles;
pub mod tree_view;

use crate::app_home::APP_HOME;
use crate::inputs;
use behavior::CmBehavior;
use behavior::CmPane;
use behavior::create_default_tree;
use eframe::egui::Align2;
use eframe::egui::Color32;
use eframe::egui::Id;
use eframe::egui::LayerId;
use eframe::egui::Order;
use eframe::egui::TextStyle;
use eframe::egui::TextureHandle;
use eframe::egui::{self};
use eyre::eyre;
use state::AppState;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

/// Run the GUI. This is async so the caller can create a runtime; the function will
/// block in place on the eframe app using `tokio::task::block_in_place`.
pub async fn run_gui() -> eyre::Result<()> {
    info!("Starting CM GUI");
    let native_options = eframe::NativeOptions::default();

    let res = tokio::task::block_in_place(move || {
        eframe::run_native(
            "CM - Creative Memories Photo Manager",
            native_options,
            Box::new(|cc| Ok(Box::new(CmApp::new(cc)))),
        )
        .map_err(|e| eyre!("Failed to run eframe: {}", e))
    });

    res?;
    info!("GUI exited");
    Ok(())
}

struct CmApp {
    tree: egui_tiles::Tree<CmPane>,
    state: AppState,
    /// Texture handle for output preview (to show cropped images)
    output_texture: Option<TextureHandle>,
    /// Path of the image currently loaded in output_texture
    output_texture_path: Option<PathBuf>,
    /// Texture handle for threshold preview
    threshold_texture: Option<TextureHandle>,
    /// Path of the image currently loaded in threshold_texture
    threshold_texture_path: Option<PathBuf>,
    /// Pan/zoom state for input preview
    input_pan_zoom: tiles::PanZoomState,
    /// Pan/zoom state for threshold preview
    threshold_pan_zoom: tiles::PanZoomState,
    /// Pan/zoom state for output preview
    output_pan_zoom: tiles::PanZoomState,
    /// Texture handles for thumbnail previews in tree view
    thumbnail_textures: HashMap<PathBuf, TextureHandle>,
}

impl CmApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        // Install image loaders for egui
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let tree = create_default_tree();
        let state = AppState::default();

        CmApp {
            tree,
            state,
            output_texture: None,
            output_texture_path: None,
            threshold_texture: None,
            threshold_texture_path: None,
            input_pan_zoom: tiles::PanZoomState::new(),
            threshold_pan_zoom: tiles::PanZoomState::new(),
            output_pan_zoom: tiles::PanZoomState::new(),
            thumbnail_textures: HashMap::new(),
        }
    }
}

impl eframe::App for CmApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Initialize on first frame
        if !self.state.initialized {
            self.state.reload_data();
            self.state.initialized = true;
        }

        // Poll background tasks for completions
        self.state.poll_background_tasks();

        // Clear textures if output info is being recalculated
        if self.state.output_info_loading {
            self.output_texture = None;
            self.output_texture_path = None;
            self.threshold_texture = None;
            self.threshold_texture_path = None;
        }

        // Handle deferred actions from previous frame
        self.state.handle_deferred_actions();

        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // Refresh button
                if ui.button("Refresh").clicked() {
                    self.state.reload_data();
                }

                // Logs toggle button
                if ui
                    .button(if self.state.logs_visible {
                        "Logs (on)"
                    } else {
                        "Logs"
                    })
                    .clicked()
                {
                    self.state.logs_visible = !self.state.logs_visible;
                    // Toggle logs tile visibility in tree
                    toggle_logs_tile(&mut self.tree, self.state.logs_visible);
                }

                // About button
                if ui.button("About").clicked() {
                    self.state.about_open = !self.state.about_open;
                }

                // Theme switch
                egui::widgets::global_theme_preference_switch(ui);

                // Right-align the Quit button
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        // Main tile panel
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut behavior = CmBehavior {
                state: &mut self.state,
                output_texture: &mut self.output_texture,
                output_texture_path: &mut self.output_texture_path,
                threshold_texture: &mut self.threshold_texture,
                threshold_texture_path: &mut self.threshold_texture_path,
                input_pan_zoom: &mut self.input_pan_zoom,
                threshold_pan_zoom: &mut self.threshold_pan_zoom,
                output_pan_zoom: &mut self.output_pan_zoom,
                thumbnail_textures: &mut self.thumbnail_textures,
            };
            self.tree.ui(&mut behavior, ui);
        });

        // Sync pan/zoom states if enabled (after drawing so dirty flags are set)
        if self.state.sync_preview_pan_zoom {
            // Find which preview was interacted with and sync others to it
            if self.input_pan_zoom.dirty {
                self.threshold_pan_zoom.sync_from(&self.input_pan_zoom);
                self.output_pan_zoom.sync_from(&self.input_pan_zoom);
            } else if self.threshold_pan_zoom.dirty {
                self.input_pan_zoom.sync_from(&self.threshold_pan_zoom);
                self.output_pan_zoom.sync_from(&self.threshold_pan_zoom);
            } else if self.output_pan_zoom.dirty {
                self.input_pan_zoom.sync_from(&self.output_pan_zoom);
                self.threshold_pan_zoom.sync_from(&self.output_pan_zoom);
            }
        }

        // Clear dirty flags for next frame
        self.input_pan_zoom.dirty = false;
        self.threshold_pan_zoom.dirty = false;
        self.output_pan_zoom.dirty = false;

        // About window
        if self.state.about_open {
            egui::Window::new("About")
                .resizable(false)
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut self.state.about_open)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("CM - Creative Memories Photo Manager");
                        ui.add_space(10.0);
                        ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                        ui.label(format!(
                            "Revision: {}",
                            option_env!("GIT_REVISION").unwrap_or("unknown")
                        ));
                        ui.add_space(10.0);
                        ui.label("An app to help manage photos of inventory items.");
                        ui.add_space(10.0);
                        ui.hyperlink_to("GitHub", "https://github.com/TeamDman/cm");
                    });
                });
        }

        // Global hover preview for files being dragged over the app
        let hovered_files = ctx.input(|i| i.raw.hovered_files.clone());
        if !hovered_files.is_empty() {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        text.push_str(&format!("\n{}", path.display()));
                    } else if !file.mime.is_empty() {
                        text.push_str(&format!("\n{}", file.mime));
                    } else {
                        text.push_str("\n???");
                    }
                }
                text
            });

            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
            let content_rect = ctx.content_rect();
            painter.rect_filled(content_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                content_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }

        // Global drop handling (adds dropped files persistently)
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            let mut added_paths = Vec::new();
            for file in dropped_files {
                if let Some(path) = file.path {
                    added_paths.push(path);
                }
            }
            if !added_paths.is_empty() {
                match inputs::add_paths(&APP_HOME, &added_paths) {
                    Ok(added) => {
                        info!("Added {} inputs", added.len());
                        self.state.reload_data();
                    }
                    Err(e) => self.state.last_error = Some(format!("{}", e)),
                }
            }
        }
    }
}

/// Toggle the logs tile visibility in the tree
fn toggle_logs_tile(tree: &mut egui_tiles::Tree<CmPane>, visible: bool) {
    // Find if we already have a logs tile
    let logs_tile_id = tree.tiles.iter().find_map(|(id, tile)| {
        if let egui_tiles::Tile::Pane(CmPane::Logs) = tile {
            Some(*id)
        } else {
            None
        }
    });

    if visible {
        // Add logs tile if not present
        if logs_tile_id.is_none() {
            let logs_id = tree.tiles.insert_pane(CmPane::Logs);
            // Add to root as a new horizontal tile
            if let Some(root_id) = tree.root()
                && let Some(egui_tiles::Tile::Container(container)) = tree.tiles.get_mut(root_id)
            {
                container.add_child(logs_id);
            }
        }
    } else {
        // Remove logs tile if present
        if let Some(id) = logs_tile_id {
            tree.tiles.remove(id);
        }
    }
}
