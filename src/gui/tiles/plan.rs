//! V2 plan tile.

use crate::app_home::APP_HOME;
use crate::gui::plan;
use crate::gui::plan::DecisionStatus;
use crate::gui::state::AppState;
use eframe::egui;

pub fn draw_plan_tile(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Plan");
    ui.separator();

    let cm_plan = plan::build_plan(state);

    ui.horizontal(|ui| {
        ui.label(format!(
            "{} ready / {} waiting / {} invalid",
            cm_plan.ready_count(),
            cm_plan.waiting_count(),
            cm_plan.invalid_count()
        ));

        if ui.button("Export").clicked() {
            if let Err(e) = plan::export_plan(&APP_HOME, &cm_plan) {
                tracing::error!("Failed to export plan: {}", e);
            } else {
                tracing::info!("Exported CM v2 plan");
            }
        }
    });

    ui.label(format!(
        "{} planned image operations",
        cm_plan.entries.len()
    ));
    if cm_plan.collision_count() > 0 {
        ui.colored_label(
            egui::Color32::YELLOW,
            format!(
                "{} output path collisions will be renamed",
                cm_plan.collision_count()
            ),
        );
    }
    if !cm_plan.errors.is_empty() {
        ui.colored_label(
            egui::Color32::RED,
            format!("{} plan issues need attention", cm_plan.errors.len()),
        );
    }

    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("plan_tile_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.collapsing("Decisions", |ui| {
                for decision in &cm_plan.decisions {
                    let color = match decision.status {
                        DecisionStatus::Waiting => egui::Color32::YELLOW,
                        DecisionStatus::Ready => egui::Color32::LIGHT_GREEN,
                        DecisionStatus::Invalid => egui::Color32::RED,
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(color, status_label(&decision.status));
                        ui.strong(&decision.name);
                        ui.label(&decision.value);
                    });
                }
            });

            ui.collapsing("Entries", |ui| {
                for entry in cm_plan.entries.iter().take(200) {
                    ui.group(|ui| {
                        ui.label(entry.input_path.display().to_string());
                        if let Some(path) = &entry.reserved_output_path {
                            ui.small(format!("-> {}", path.display()));
                        }
                        for transformation in &entry.transformations {
                            ui.small(format!(
                                "{}: {}",
                                transformation.name, transformation.detail
                            ));
                        }
                        for expectation in &entry.expectations {
                            ui.small(format!("expect: {expectation}"));
                        }
                    });
                }
                if cm_plan.entries.len() > 200 {
                    ui.label(format!(
                        "{} more entries omitted from tile; export the plan for the full list",
                        cm_plan.entries.len() - 200
                    ));
                }
            });

            if !cm_plan.errors.is_empty() {
                ui.collapsing("Issues", |ui| {
                    for error in &cm_plan.errors {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                });
            }
        });
}

fn status_label(status: &DecisionStatus) -> &'static str {
    match status {
        DecisionStatus::Waiting => "waiting",
        DecisionStatus::Ready => "ready",
        DecisionStatus::Invalid => "invalid",
    }
}
