//! Max name length tile - global setting for maximum file name length

use crate::MAX_NAME_LENGTH;
use crate::MaxNameLength;
use crate::MaxNameLengthEnforced;
use crate::app_home::APP_HOME;
use crate::egui::state::AppState;
use eframe::egui;
use std::sync::atomic::Ordering;
use tracing::error;

/// Draw the max name length tile UI
#[expect(clippy::cast_possible_truncation)]
pub fn draw_max_name_length_tile(ui: &mut egui::Ui, state: &mut AppState) {
    if ui
        .checkbox(
            &mut state.max_name_length_enforced,
            "Warn about long file names",
        )
        .changed()
    {
        if let Err(e) = MaxNameLengthEnforced::set_to(&APP_HOME, state.max_name_length_enforced) {
            error!("Failed to save max-name enforcement: {}", e);
        }
        state.rename_preview_key = 0;
    }

    ui.label("Maximum file name length:");
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        let mut value = state.max_name_length as u32;
        if ui
            .add_enabled(
                state.max_name_length_enforced,
                egui::DragValue::new(&mut value).range(10..=500).speed(1.0),
            )
            .changed()
        {
            state.max_name_length = value as usize;
            // Persist to disk and update global
            if let Err(e) = MaxNameLength::set_to(&APP_HOME, value as usize) {
                error!("Failed to save max name length: {}", e);
            }
            state.rename_preview_key = 0; // Invalidate cache
        }

        ui.label("characters");
    });

    ui.add_space(8.0);

    // Show current value info
    let current = MAX_NAME_LENGTH.load(Ordering::SeqCst);
    ui.label(format!("Current global value: {current}"));

    ui.add_space(8.0);

    // Reset to default button
    if ui
        .add_enabled(
            state.max_name_length != MaxNameLength::DEFAULT,
            egui::Button::new("Reset to default (50)"),
        )
        .clicked()
    {
        state.max_name_length = MaxNameLength::DEFAULT;
        if let Err(e) = MaxNameLength::set_to(&APP_HOME, MaxNameLength::DEFAULT) {
            error!("Failed to reset max name length: {}", e);
        }
        state.rename_preview_key = 0;
    }

    ui.add_space(16.0);

    // Explanation
    if state.max_name_length_enforced {
        ui.label(
            "Files with names longer than this limit will be shown in red in the output preview.",
        );
    } else {
        ui.label("Long file-name warnings are disabled.");
    }
    ui.add_space(4.0);
    ui.label("Rules with 'only when name too long' checked will only apply to files exceeding this length.");
}
