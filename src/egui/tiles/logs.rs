//! Logs tile - shows tracing logs

use eframe::egui;

/// Draw the logs tile UI
pub fn draw_logs_tile(ui: &mut egui::Ui) {
    let collector = crate::tracing::event_collector();
    ui.add(egui_tracing::Logs::new(collector));
}
