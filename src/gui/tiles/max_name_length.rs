//! Max name length tile - global setting for maximum file name length

use crate::gui::state::AppState;
use crate::MaxNameLength;
use crate::MAX_NAME_LENGTH;
use eframe::egui;
use std::sync::atomic::Ordering;

/// Draw the max name length tile UI
pub fn draw_max_name_length_tile(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label("Maximum file name length:");
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        let mut value = state.max_name_length as u32;
        if ui
            .add(egui::DragValue::new(&mut value).range(10..=500).speed(1.0))
            .changed()
        {
            state.max_name_length = value as usize;
            // Persist to disk and update global
            if let Err(e) = MaxNameLength::set_to(value as usize) {
                state.last_error = Some(format!("Failed to save max name length: {}", e));
            }
            state.rename_preview_key = 0; // Invalidate cache
        }

        ui.label("characters");
    });

    ui.add_space(8.0);

    // Show current value info
    let current = MAX_NAME_LENGTH.load(Ordering::SeqCst);
    ui.label(format!("Current global value: {}", current));

    ui.add_space(8.0);

    // Reset to default button
    if ui.button("Reset to default (50)").clicked() {
        state.max_name_length = MaxNameLength::DEFAULT;
        if let Err(e) = MaxNameLength::set_to(MaxNameLength::DEFAULT) {
            state.last_error = Some(format!("Failed to reset max name length: {}", e));
        }
        state.rename_preview_key = 0;
    }

    ui.add_space(16.0);

    // Explanation
    ui.label("Files with names longer than this limit will be shown in red in the output preview.");
    ui.add_space(4.0);
    ui.label("Rules with 'only when name too long' checked will only apply to files exceeding this length.");
}
