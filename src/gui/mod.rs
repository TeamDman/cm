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
use eframe::egui::Color32;
use eframe::egui::ScrollArea;
use eframe::egui::{self};
use std::path::PathBuf;

#[derive(Default)]
struct SimpleApp {
    inputs: Vec<String>,
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
                    // not destructive: just clears the view; persistence not changed
                    self.inputs.clear();
                    self.last_error = None;
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
    }
}
