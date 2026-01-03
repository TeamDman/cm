use eyre::eyre;
use tracing::info;

/// Run the GUI. This is async so the caller can create a runtime; the function will
/// block in place on the eframe app using `tokio::task::block_in_place`.
pub async fn run_gui() -> eyre::Result<()> {
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
    last_error: Option<String>,
    last_message: Option<String>,
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
                            self.last_message = Some("Cleared all persisted inputs".to_string());
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

            if let Some(msg) = &self.last_message {
                ui.colored_label(Color32::LIGHT_GREEN, msg);
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
            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
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
                        self.last_message = Some(format!("Added {} inputs", added.len()));
                        self.reload_inputs();
                    }
                    Err(e) => self.last_error = Some(format!("{}", e)),
                }
            }
        }
    }
}
