use eyre::eyre;
use tracing::info;

/// Run the GUI. This is async so the caller can create a runtime; the function will
/// block in place on the eframe app using `tokio::task::block_in_place`.
pub async fn run_gui() -> eyre::Result<()> {
    info!("hi");
    // Basic native options; keep it minimal for now.
    let native_options = eframe::NativeOptions::default();

    // Use block_in_place since eframe is synchronous and needs to run on the current thread
    let res = tokio::task::block_in_place(move || {
        eframe::run_native(
            "cm GUI",
            native_options,
            Box::new(|_cc| Ok(Box::new(SimpleApp::default()))),
        )
        .map_err(|e| eyre!("Failed to run eframe: {}", e))
    });

    res?;
    info!("GUI exited");
    Ok(())
}

use crate::app_home::APP_HOME;
use crate::inputs;
use eframe::egui::{self, Align2, Color32, Id, LayerId, Order, ScrollArea, TextStyle};
use std::path::PathBuf;

#[derive(Default)]
struct SimpleApp {
    inputs: Vec<String>,
    files: Vec<PathBuf>,
    last_error: Option<String>,
    loaded: bool,
} 

impl SimpleApp {
    fn reload_inputs(&mut self) {
        match inputs::load_inputs(&APP_HOME) {
            Ok(v) => {
                self.inputs = v
                    .into_iter()
                    .map(|p: PathBuf| p.display().to_string())
                    .collect();
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("{}", e));
                self.inputs.clear();
            }
        }

        // Update files list too (non-fatal)
        match inputs::list_files(&APP_HOME) {
            Ok(mut f) => {
                f.sort();
                self.files = f;
            }
            Err(e) => {
                // don't clobber earlier error, but note it
                if self.last_error.is_none() {
                    self.last_error = Some(format!("{}", e));
                }
                self.files.clear();
            }
        }
    }
}

impl eframe::App for SimpleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.loaded {
            self.reload_inputs();
            self.loaded = true;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("CM GUI");
            ui.label("Use the Inputs window to view persisted inputs (same as `cm input list`).");
        });

        egui::Window::new("Inputs").resizable(true).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Refresh").clicked() {
                    self.reload_inputs();
                }
                if ui.button("Clear").clicked() {
                    // Destructive: remove all persisted inputs
                    match inputs::clear_all(&APP_HOME) {
                        Ok(()) => {
                            self.reload_inputs();
                            info!("Cleared all persisted inputs");
                            self.last_error = None;
                        }
                        Err(e) => {
                            self.last_error = Some(format!("{}", e));
                        }
                    }
                }
            });

            ui.separator();

            if let Some(err) = &self.last_error {
                ui.colored_label(Color32::RED, format!("Error loading inputs: {}", err));
                return;
            }


            if self.inputs.is_empty() {
                ui.label("(no persisted inputs)");
                return;
            }

            ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for p in &self.inputs {
                    ui.label(p);
                }
            });

        });

        // Input Files window: show files derived from inputs (files in dirs and files directly added)
        egui::Window::new("Input Files").resizable(true).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Refresh").clicked() {
                    self.reload_inputs();
                }
            });
            ui.separator();

            if self.files.is_empty() {
                ui.label("(no files from inputs)");
                return;
            }

            ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for p in &self.files {
                    // highlight image files green
                    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                        match ext.to_ascii_lowercase().as_str() {
                            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" => {
                                ui.colored_label(Color32::LIGHT_GREEN, p.display().to_string());
                            }
                            _ => {
                                ui.label(p.display().to_string());
                            }
                        }
                    } else {
                        ui.label(p.display().to_string());
                    }
                }
            });
        });

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

            let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
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
            let mut added_paths: Vec<PathBuf> = Vec::new();
            for file in dropped_files {
                if let Some(path) = file.path {
                    added_paths.push(path);
                }
            }
            if !added_paths.is_empty() {
                match inputs::add_paths(&APP_HOME, &added_paths) {
                    Ok(added) => {
                        info!("Added {} inputs", added.len());
                        self.reload_inputs();
                    }
                    Err(e) => self.last_error = Some(format!("{}", e)),
                }
            }
        }

        // Logs Control removed — logs are always visible.

        // Render logs window (always visible)
        egui::Window::new("Logs").resizable(true).show(ctx, |ui| {
            let collector = crate::tracing::event_collector();
            ui.add(egui_tracing::Logs::new(collector));
        });
    }
}
