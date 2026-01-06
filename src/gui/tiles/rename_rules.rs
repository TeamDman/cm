//! Rename rules tile - shows and manages rename rules

use crate::app_home::APP_HOME;
use crate::gui::state::AppState;
use eframe::egui::{self, ScrollArea};

/// Draw the rename rules tile UI
pub fn draw_rename_rules_tile(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label("Find & Replace Rules:");
    ui.add_space(4.0);

    ScrollArea::vertical()
        .id_salt("rules_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut rules_to_remove = Vec::new();
            let mut rules_changed = Vec::new();

            for rule in state.rename_rules.iter_mut() {
                let mut rule_changed = false;

                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("✖").clicked() {
                            rules_to_remove.push(rule.id);
                        }

                        ui.label("Find:");
                        if ui
                            .add(egui::TextEdit::singleline(&mut rule.find).desired_width(150.0))
                            .changed()
                        {
                            rule_changed = true;
                        }
                        ui.label("Replace:");
                        if ui
                            .add(egui::TextEdit::singleline(&mut rule.replace).desired_width(150.0))
                            .changed()
                        {
                            rule_changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut rule.enabled, "enabled").changed() {
                            rule_changed = true;
                        }

                        if ui.checkbox(&mut rule.case_sensitive, "case sensitive").changed() {
                            rule_changed = true;
                        }

                        if ui
                            .checkbox(&mut rule.only_when_name_too_long, "only when name too long")
                            .changed()
                        {
                            rule_changed = true;
                        }
                    });
                });

                if rule_changed {
                    rules_changed.push(rule.clone());
                }

                ui.add_space(4.0);
            }

            // Apply changes
            for id in rules_to_remove {
                let _ = crate::rename_rules::remove_rule(&APP_HOME, id);
                state.rename_rules.retain(|r| r.id != id);
                state.rename_preview_key = 0; // Invalidate cache
            }

            for rule in rules_changed {
                let _ = crate::rename_rules::write_rule(&APP_HOME, &rule);
                state.rename_preview_key = 0; // Invalidate cache
            }

            ui.add_space(8.0);

            if ui.button("+ Add Rule").clicked() {
                let new_rule = crate::rename_rules::RenameRule::default();
                if crate::rename_rules::add_rule(&APP_HOME, &new_rule).is_ok() {
                    state.rename_rules.push(new_rule);
                    state.rename_preview_key = 0;
                }
            }
        });
}
