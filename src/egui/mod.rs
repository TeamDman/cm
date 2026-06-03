//! CM GUI using `egui_tiles` for layout management

mod behavior;
mod folder_picker;
mod layouts;
pub mod state;
mod tiles;
pub mod tree_view;

use crate::app_home::APP_HOME;
use crate::egui::layouts::Layout;
use crate::egui::layouts::LayoutManager;
use crate::inputs;
use behavior::CmBehavior;
use behavior::CmPane;
use behavior::create_default_tree;
use behavior::create_product_search_tree;
use behavior::create_v2_tree;
use eframe::egui::Align2;
use eframe::egui::Color32;
use eframe::egui::Id;
use eframe::egui::LayerId;
use eframe::egui::Order;
use eframe::egui::TextStyle;
use eframe::egui::TextureHandle;
use eframe::egui::{self};
use egui_toast::Toast;
use egui_toast::ToastKind;
use egui_toast::ToastOptions;
use egui_toast::Toasts;
use eyre::eyre;
use state::AppState;
use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;
use tracing::Level;
use tracing::debug;
use tracing::error;
use tracing::info;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolChoice {
    V1,
    V2,
    ProductSearch,
}

impl ToolChoice {
    fn label(self) -> &'static str {
        match self {
            ToolChoice::V1 => "Tool v1",
            ToolChoice::V2 => "Tool v2",
            ToolChoice::ProductSearch => "Product Search",
        }
    }

    fn button_label(self) -> &'static str {
        match self {
            ToolChoice::V1 => "Run Tool v1",
            ToolChoice::V2 => "Run Tool v2",
            ToolChoice::ProductSearch => "Product Search",
        }
    }

    fn uses_studio_layouts(self) -> bool {
        matches!(self, ToolChoice::V1)
    }
}

/// Run the GUI; the function blocks in place on the eframe app using
/// `tokio::task::block_in_place`.
/// # Errors
/// Returns an error if the GUI fails to start or run.
/// # Panics
/// Panics if the blocking GUI task returns an error after startup.
pub fn run_gui() -> eyre::Result<()> {
    run_gui_with_initial_tool(None)
}

/// Run the GUI with an optional starting tool. Supplying a tool skips the
/// main menu and opens that surface directly.
/// # Errors
/// Returns an error if the GUI fails to start or run.
/// # Panics
/// Panics if the blocking GUI task returns an error after startup.
pub(crate) fn run_gui_with_initial_tool(initial_tool: Option<ToolChoice>) -> eyre::Result<()> {
    info!("Starting CM GUI");
    // Create a dedicated runtime and run the GUI
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let native_options = eframe::NativeOptions::default();

        let res = tokio::task::block_in_place(move || {
            eframe::run_native(
                "CM - Creative Memories Photo Manager",
                native_options,
                Box::new(move |cc| Ok(Box::new(CmApp::new(cc, initial_tool)))),
            )
            .map_err(|e| eyre!("Failed to run eframe: {}", e))
        });

        res.expect("GUI runtime failed");

        info!("GUI exited");
        debug!("Finishing process aggressively, any unfinished tasks are pooched");
        std::process::exit(0);
    });
    Ok(())
}

struct CmApp {
    tool_choice: Option<ToolChoice>,
    main_menu_status: Option<String>,
    tree: egui_tiles::Tree<CmPane>,
    state: AppState,
    /// Texture handle for output preview (to show cropped images)
    output_texture: Option<TextureHandle>,
    /// Path of the image currently loaded in `output_texture`
    output_texture_path: Option<PathBuf>,
    /// Texture handle for threshold preview
    threshold_texture: Option<TextureHandle>,
    /// Path of the image currently loaded in `threshold_texture`
    threshold_texture_path: Option<PathBuf>,
    /// Pan/zoom state for input preview
    input_pan_zoom: tiles::PanZoomState,
    /// Pan/zoom state for threshold preview
    threshold_pan_zoom: tiles::PanZoomState,
    /// Pan/zoom state for output preview
    output_pan_zoom: tiles::PanZoomState,
    /// Texture handles for thumbnail previews in tree view
    thumbnail_textures: HashMap<PathBuf, TextureHandle>,
    /// Toast notifications manager
    toasts: Toasts,
    /// Number of events we've already processed for toasts
    last_seen_event_count: usize,
    /// Layout manager (persistence + active layout)
    layout_manager: LayoutManager,
}

impl CmApp {
    fn new(cc: &eframe::CreationContext, initial_tool: Option<ToolChoice>) -> Self {
        // Install image loaders for egui
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let tree = create_default_tree();
        let state = AppState::default();

        // Initialize layout manager and ensure we have at least one preset and one custom
        let mut layout_manager = LayoutManager::new();
        // If no presets exist, save a "Preset 1" based on current tree
        if layout_manager.list_presets().is_empty()
            && let Some(mut preset_layout) = Layout::from_tree(&tree)
        {
            preset_layout.name = "Preset 1".to_string();
            let _ = layout_manager.save_preset("Preset 1", &preset_layout);
        }
        // If no custom layouts exist, create a Custom 1 from the first preset
        if layout_manager.list_custom().is_empty()
            && let Some(preset_name) = layout_manager.list_presets().first().cloned()
            && let Ok(new_name) = layout_manager.activate_preset_as_custom(&preset_name, tree.id())
        {
            layout_manager.set_active(&new_name);
        }

        // Get current event count so we don't show toasts for old events
        let initial_event_count = crate::tracing::event_collector().events().len();

        CmApp {
            tool_choice: initial_tool,
            main_menu_status: None,
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
            toasts: Toasts::new()
                .anchor(Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
            last_seen_event_count: initial_event_count,
            layout_manager,
        }
    }
}

impl eframe::App for CmApp {
    #[expect(clippy::too_many_lines)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.tool_choice.is_none() {
            draw_main_menu(ctx, &mut self.tool_choice, &mut self.main_menu_status);
            return;
        }

        // Initialize on first frame
        if !self.state.initialized {
            self.tree = match self.tool_choice.expect("tool choice is selected") {
                ToolChoice::V1 => create_default_tree(),
                ToolChoice::V2 => create_v2_tree(),
                ToolChoice::ProductSearch => create_product_search_tree(),
            };
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
        let selected_tool = self.tool_choice.expect("tool choice is selected");
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // Refresh button
                if ui.button("Refresh").clicked() {
                    self.state.reload_data();
                }

                ui.label(selected_tool.label());

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
                }

                // About button
                if ui.button("About").clicked() {
                    self.state.about_open = !self.state.about_open;
                }

                if selected_tool.uses_studio_layouts() {
                    // Layout menu
                    ui.menu_button("Layout", |ui| {
                        // Custom layouts (active shown)
                        let customs = self.layout_manager.list_custom();
                        if customs.is_empty() {
                            if ui.button("No custom layout").clicked() {}
                        } else {
                            for name in customs {
                                if Some(name.as_str()) == self.layout_manager.active_name() {
                                    ui.label(format!("{name} (active)"));
                                } else if ui.button(&name).clicked()
                                    && let Ok(layout) = self.layout_manager.load_named(&name)
                                {
                                    self.tree = layout.apply_to_tree(self.tree.id());
                                    self.layout_manager.set_active(&name);
                                }
                            }
                        }

                        ui.separator();

                        // Presets
                        for preset in self.layout_manager.list_presets() {
                            if ui.button(&preset).clicked()
                                && let Ok(new_name) = self
                                    .layout_manager
                                    .activate_preset_as_custom(&preset, self.tree.id())
                                && let Ok(layout) = self.layout_manager.load_named(&new_name)
                            {
                                self.tree = layout.apply_to_tree(self.tree.id());
                                self.layout_manager.set_active(&new_name);
                            }
                        }

                        ui.separator();

                        if ui.button("Create New").clicked()
                            && let Some(layout) = Layout::from_tree(&self.tree)
                        {
                            let name =
                                format!("Custom {}", self.layout_manager.list_custom().len() + 1);
                            if let Ok(new_name) = self
                                .layout_manager
                                .create_custom_from_layout(&name, &layout)
                            {
                                self.layout_manager.set_active(&new_name);
                            }
                        }

                        if ui.button("Delete Active").clicked() {
                            let _ = self.layout_manager.delete_active();
                        }
                    });
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

            // Autosave active layout if tree changed
            if selected_tool.uses_studio_layouts()
                && let Some(layout) = Layout::from_tree(&self.tree)
            {
                let _ = self.layout_manager.maybe_autosave(&layout);
            }
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

        // Logs window (separate window instead of tile)
        if self.state.logs_visible {
            egui::Window::new("Logs")
                .default_size([800.0, 400.0])
                .open(&mut self.state.logs_visible)
                .show(ctx, |ui| {
                    tiles::draw_logs_tile(ui);
                });
        }

        // Process new log events and create toasts for INFO and ERROR levels
        let collector = crate::tracing::event_collector();
        let events = collector.events();
        let new_events = &events[self.last_seen_event_count..];
        for event in new_events {
            let kind = match event.level {
                Level::INFO => Some(ToastKind::Info),
                Level::ERROR => Some(ToastKind::Error),
                _ => None,
            };
            if let Some(kind) = kind {
                let message = event
                    .fields
                    .get("message")
                    .map_or("", std::string::String::as_str)
                    .to_string();
                self.toasts.add(
                    Toast::default().kind(kind).text(message).options(
                        ToastOptions::default()
                            .duration_in_seconds(5.0)
                            .show_progress(true)
                            .show_icon(true),
                    ),
                );
            }
        }
        self.last_seen_event_count = events.len();

        // Show toasts
        self.toasts.show(ctx);

        // Global hover preview for files being dragged over the app
        let hovered_files = ctx.input(|i| i.raw.hovered_files.clone());
        if !hovered_files.is_empty() {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        let _ = write!(text, "\n{}", path.display());
                    } else if !file.mime.is_empty() {
                        let _ = write!(text, "\n{}", file.mime);
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
                    Err(e) => error!("{}", e),
                }
            }
        }
    }
}

fn draw_main_menu(
    ctx: &egui::Context,
    tool_choice: &mut Option<ToolChoice>,
    main_menu_status: &mut Option<String>,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.3).min(160.0));
            ui.heading("CM");
            ui.add_space(24.0);

            ui.spacing_mut().item_spacing.y = 12.0;
            for choice in [ToolChoice::V1, ToolChoice::V2, ToolChoice::ProductSearch] {
                let button =
                    egui::Button::new(choice.button_label()).min_size(egui::vec2(240.0, 44.0));
                if ui.add(button).clicked() {
                    *tool_choice = Some(choice);
                    *main_menu_status = None;
                    info!("Selected {}", choice.label());
                }
            }

            if let Some(status) = main_menu_status {
                ui.add_space(12.0);
                ui.label(status.as_str());
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::ToolChoice;

    #[test]
    fn product_search_is_a_main_menu_utility_not_a_studio_layout() {
        assert_eq!(ToolChoice::ProductSearch.button_label(), "Product Search");
        assert_eq!(ToolChoice::ProductSearch.label(), "Product Search");
        assert!(!ToolChoice::ProductSearch.uses_studio_layouts());
    }

    #[test]
    fn only_v1_uses_egui_studio_layouts() {
        assert!(ToolChoice::V1.uses_studio_layouts());
        assert!(!ToolChoice::V2.uses_studio_layouts());
    }
}
