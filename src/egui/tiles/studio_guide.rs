//! V2 studio guide tile.

use crate::MaxNameLength;
use crate::MaxNameLengthEnforced;
use crate::app_home::APP_HOME;
use crate::egui::folder_picker;
use crate::egui::state::AppState;
use crate::egui::state::StudioStep;
use crate::egui::tree_view::open_in_explorer;
use crate::image_processing;
use crate::image_processing::BinarizationMode;
use crate::image_processing::DEFAULT_MAX_FILE_SIZE_BYTES;
use crate::inputs;
use crate::reactor::plan;
use crate::reactor::plan::DecisionStatus;
use eframe::egui;
use std::path::Path;
use std::path::PathBuf;

#[expect(clippy::too_many_lines)]
pub fn draw_studio_guide_tile(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Studio Guide");
    ui.separator();

    let cm_plan = plan::build_plan(state);
    let can_execute = !cm_plan.entries.is_empty() && cm_plan.errors.is_empty();

    ui.horizontal_wrapped(|ui| {
        ui.label(format!("{} images", cm_plan.entries.len()));
        ui.separator();
        ui.label(format!("{} ready", cm_plan.ready_count()));
        ui.separator();
        ui.label(format!("{} waiting", cm_plan.waiting_count()));
        if !cm_plan.errors.is_empty() {
            ui.separator();
            ui.colored_label(
                egui::Color32::RED,
                format!("{} issues", cm_plan.errors.len()),
            );
        }
        if cm_plan.collision_count() > 0 {
            ui.separator();
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("{} renamed outputs", cm_plan.collision_count()),
            );
        }
    });

    ui.add_space(8.0);

    draw_step_rail(ui, state, &cm_plan, can_execute);
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("studio_guide_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            draw_active_step(ui, state, &cm_plan, can_execute);
            draw_navigation(ui, state);

            if !cm_plan.errors.is_empty() {
                ui.separator();
                ui.collapsing("Plan issues", |ui| {
                    for error in &cm_plan.errors {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                });
            }
        });
}

fn draw_step_rail(ui: &mut egui::Ui, state: &mut AppState, plan: &plan::CmPlan, can_execute: bool) {
    ui.horizontal_wrapped(|ui| {
        for step in StudioStep::ALL {
            let status = step_status(plan, step, can_execute);
            let is_current = state.studio_step == step;
            let label = format!("{}. {}", step.number(), step.label());
            let text = if is_current {
                egui::RichText::new(label).strong()
            } else {
                egui::RichText::new(label)
            }
            .color(status_color(&status));

            if ui.selectable_label(is_current, text).clicked() {
                state.studio_step = step;
            }
        }
    });
}

fn draw_active_step(
    ui: &mut egui::Ui,
    state: &mut AppState,
    plan: &plan::CmPlan,
    can_execute: bool,
) {
    match state.studio_step {
        StudioStep::PickPhotos => draw_step(
            ui,
            &step_title(StudioStep::PickPhotos),
            decision_value(plan, "input paths"),
            decision_status(plan, "input paths"),
            |ui| draw_pick_photos_actions(ui, state),
        ),
        StudioStep::ReviewImages => draw_step(
            ui,
            &step_title(StudioStep::ReviewImages),
            decision_value(plan, "input image paths"),
            decision_status(plan, "input image paths"),
            |ui| draw_review_images_actions(ui, state),
        ),
        StudioStep::OutputShape => draw_step(
            ui,
            &step_title(StudioStep::OutputShape),
            decision_value(plan, "output directory strategy"),
            decision_status(plan, "output directory strategy"),
            |ui| draw_output_shape_actions(ui, state),
        ),
        StudioStep::Processing => draw_step(
            ui,
            &step_title(StudioStep::Processing),
            format!(
                "{}, {}",
                decision_value(plan, "crop images to content"),
                decision_value(plan, "reduce file size")
            ),
            merged_status(
                decision_status(plan, "crop images to content"),
                decision_status(plan, "reduce file size"),
            ),
            |ui| draw_processing_actions(ui, state),
        ),
        StudioStep::Naming => draw_step(
            ui,
            &step_title(StudioStep::Naming),
            format!(
                "{}, {}",
                decision_value(plan, "rename images"),
                decision_value(plan, "max file name length")
            ),
            merged_status(
                decision_status(plan, "rename images"),
                decision_status(plan, "max file name length"),
            ),
            |ui| draw_name_actions(ui, state),
        ),
        StudioStep::Run => draw_step(
            ui,
            &step_title(StudioStep::Run),
            if can_execute {
                "ready to process".to_string()
            } else {
                "waiting for a valid plan".to_string()
            },
            if can_execute {
                DecisionStatus::Ready
            } else {
                DecisionStatus::Waiting
            },
            |ui| draw_run_actions(ui, state, plan, can_execute),
        ),
    }
}

fn draw_navigation(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                state.studio_step.previous().is_some(),
                egui::Button::new("Back"),
            )
            .clicked()
            && let Some(previous) = state.studio_step.previous()
        {
            state.studio_step = previous;
        }

        if ui
            .add_enabled(
                state.studio_step.next().is_some(),
                egui::Button::new("Next"),
            )
            .clicked()
            && let Some(next) = state.studio_step.next()
        {
            state.studio_step = next;
        }
    });
}

fn draw_step(
    ui: &mut egui::Ui,
    title: &str,
    value: String,
    status: DecisionStatus,
    actions: impl FnOnce(&mut egui::Ui),
) {
    let color = match status {
        DecisionStatus::Waiting => egui::Color32::YELLOW,
        DecisionStatus::Ready => egui::Color32::LIGHT_GREEN,
        DecisionStatus::Invalid => egui::Color32::RED,
    };

    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(color, status_label(&status));
            ui.strong(title);
        });
        ui.label(value);
        ui.add_space(6.0);
        actions(ui);
    });
    ui.add_space(6.0);
}

fn step_title(step: StudioStep) -> String {
    format!("{}. {}", step.number(), step.label())
}

fn step_status(plan: &plan::CmPlan, step: StudioStep, can_execute: bool) -> DecisionStatus {
    match step {
        StudioStep::PickPhotos => decision_status(plan, "input paths"),
        StudioStep::ReviewImages => decision_status(plan, "input image paths"),
        StudioStep::OutputShape => decision_status(plan, "output directory strategy"),
        StudioStep::Processing => merged_status(
            decision_status(plan, "crop images to content"),
            decision_status(plan, "reduce file size"),
        ),
        StudioStep::Naming => merged_status(
            decision_status(plan, "rename images"),
            decision_status(plan, "max file name length"),
        ),
        StudioStep::Run => {
            if can_execute {
                DecisionStatus::Ready
            } else {
                DecisionStatus::Waiting
            }
        }
    }
}

fn status_color(status: &DecisionStatus) -> egui::Color32 {
    match status {
        DecisionStatus::Waiting => egui::Color32::YELLOW,
        DecisionStatus::Ready => egui::Color32::LIGHT_GREEN,
        DecisionStatus::Invalid => egui::Color32::RED,
    }
}

fn draw_pick_photos_actions(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("Add folder").clicked()
            && let Some(path) = folder_picker::pick_folder()
        {
            add_input_paths(state, vec![path]);
        }
        if ui.button("Add file").clicked()
            && let Some(path) = folder_picker::pick_file()
        {
            add_input_paths(state, vec![path]);
        }
        if ui.button("Refresh").clicked() {
            state.reload_data();
        }
        if ui
            .add_enabled(
                !state.input_paths.is_empty(),
                egui::Button::new("Clear all"),
            )
            .clicked()
        {
            state.clear_all = true;
        }
    });
    draw_current_input_paths(ui, state);
    draw_recent_input_paths(ui, state);
    ui.small("Files and folders can also be dragged into the app window.");
}

fn draw_current_input_paths(ui: &mut egui::Ui, state: &mut AppState) {
    if state.input_paths_loading.is_loading() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.small("Loading input paths...");
        });
        return;
    }

    if state.input_paths.is_empty() {
        return;
    }

    ui.collapsing("Selected inputs", |ui| {
        egui::ScrollArea::vertical()
            .id_salt("studio_guide_selected_inputs")
            .max_height(160.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for path in state.input_paths.iter().cloned().collect::<Vec<_>>() {
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("Remove").clicked() {
                            state.path_to_remove = Some(path.clone());
                        }
                        ui.label(path_button_label(&path))
                            .on_hover_text(path.display().to_string());
                    });
                }
            });
    });
}

fn draw_recent_input_paths(ui: &mut egui::Ui, state: &mut AppState) {
    if state.recent_input_paths.is_empty() {
        return;
    }

    ui.horizontal_wrapped(|ui| {
        ui.small("Recent:");
        for path in state
            .recent_input_paths
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
        {
            let label = path.file_name().map_or_else(
                || path.display().to_string(),
                |name| name.to_string_lossy().to_string(),
            );
            let exists = path.exists();
            let already_selected = state
                .input_paths
                .iter()
                .any(|input_path| input_path == &path);
            let hover_text = if exists {
                path.display().to_string()
            } else {
                format!("Missing: {}", path.display())
            };

            if ui
                .add_enabled(
                    exists && !already_selected,
                    egui::Button::new(label).small(),
                )
                .on_hover_text(hover_text)
                .clicked()
            {
                add_input_paths(state, vec![path]);
            }
        }
    });
}

fn draw_review_images_actions(ui: &mut egui::Ui, state: &mut AppState) {
    if let Some(selected) = &state.selected_input_file {
        ui.small(format!("Selected: {}", selected.display()));
    }

    draw_review_image_picker(ui, state);

    if let Some(output_path) = &state.output_preview_path {
        ui.small(format!("Preview output: {}", output_path.display()));
    }

    if state.selected_input_file.is_some() {
        ui.collapsing("Selected image metadata", |ui| {
            super::draw_image_description_tile(ui, state);
        });
    }
}

fn draw_review_image_picker(ui: &mut egui::Ui, state: &mut AppState) {
    if state.image_files_loading.is_loading() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.small("Discovering images...");
        });
        return;
    }

    if state.image_files.is_empty() {
        ui.small("Image discovery starts after at least one input path is picked.");
        return;
    }

    ui.collapsing("Discovered images", |ui| {
        egui::ScrollArea::vertical()
            .id_salt("studio_guide_discovered_images")
            .max_height(220.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for path in state
                    .image_files
                    .iter()
                    .take(200)
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    let is_selected = state.selected_input_file.as_ref() == Some(&path);
                    if ui
                        .selectable_label(is_selected, path_button_label(&path))
                        .on_hover_text(path.display().to_string())
                        .clicked()
                    {
                        state.select_file(&path);
                    }
                }

                if state.image_files.len() > 200 {
                    ui.small(format!(
                        "{} more images are visible in the Input Images pane",
                        state.image_files.len() - 200
                    ));
                }
            });
    });
}

fn draw_output_shape_actions(ui: &mut egui::Ui, state: &mut AppState) {
    let mut output_changed = false;

    output_changed |= ui
        .checkbox(
            &mut state.flatten_output_hierarchy,
            "Flatten folders in output",
        )
        .changed();
    output_changed |= ui
        .checkbox(
            &mut state.save_all_inputs_to_same_folder,
            "Use one output folder",
        )
        .changed();

    if state.save_all_inputs_to_same_folder {
        ui.horizontal(|ui| {
            ui.label("Output folder:");
            let effective_dir = state.effective_shared_output_dir();
            let mut path_text = effective_dir
                .as_ref()
                .map_or_else(String::new, |path| path.display().to_string());
            let response = ui.add(
                egui::TextEdit::singleline(&mut path_text)
                    .desired_width(ui.available_width().max(240.0)),
            );
            if response.changed() {
                let trimmed = path_text.trim();
                state.shared_output_dir = if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.into())
                };
                output_changed = true;
            }
        });

        ui.horizontal_wrapped(|ui| {
            if state.shared_output_dir.is_none() {
                ui.small("Using default output folder");
            } else if ui.button("Use default").clicked() {
                state.shared_output_dir = None;
                output_changed = true;
            }

            if ui.button("Browse").clicked()
                && let Some(path) = folder_picker::pick_folder()
            {
                remember_shared_output_dir(state, path.clone());
                state.shared_output_dir = Some(path);
                output_changed = true;
            }

            let effective_dir = state.effective_shared_output_dir();
            if ui
                .add_enabled(
                    state.shared_output_dir.is_some(),
                    egui::Button::new("Remember"),
                )
                .clicked()
                && let Some(path) = state.shared_output_dir.clone()
            {
                remember_shared_output_dir(state, path);
            }
            if ui
                .add_enabled(
                    effective_dir.as_ref().is_some_and(|path| !path.exists()),
                    egui::Button::new("Create"),
                )
                .clicked()
                && let Some(path) = effective_dir.as_ref()
            {
                if let Err(e) = std::fs::create_dir_all(path) {
                    tracing::error!("Failed to create output folder {}: {}", path.display(), e);
                }
            }
            if ui
                .add_enabled(
                    effective_dir.as_ref().is_some_and(|path| path.is_dir()),
                    egui::Button::new("Open"),
                )
                .clicked()
                && let Some(path) = effective_dir.as_ref()
            {
                if path.is_dir() {
                    open_in_explorer(path);
                }
            }
        });

        draw_shared_output_dir_status(ui, state.effective_shared_output_dir().as_deref());
        draw_recent_output_dirs(ui, state, &mut output_changed);
        draw_suggested_output_dirs(ui, state, &mut output_changed);
    }

    if output_changed {
        refresh_selected_paths(state);
    }
}

fn draw_shared_output_dir_status(ui: &mut egui::Ui, output_dir: Option<&Path>) {
    let Some(output_dir) = output_dir else {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Output folder is waiting for input paths.",
        );
        return;
    };

    if output_dir.is_dir() {
        ui.small(format!("Output folder exists: {}", output_dir.display()));
    } else if output_dir.exists() {
        ui.colored_label(
            egui::Color32::RED,
            format!("Output path is not a folder: {}", output_dir.display()),
        );
    } else {
        ui.small(format!(
            "Output folder will be created: {}",
            output_dir.display()
        ));
    }
}

fn draw_recent_output_dirs(ui: &mut egui::Ui, state: &mut AppState, output_changed: &mut bool) {
    if state.recent_output_dirs.is_empty() {
        return;
    }

    ui.horizontal_wrapped(|ui| {
        ui.small("Recent:");
        for path in state
            .recent_output_dirs
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
        {
            let label = path_button_label(&path);
            if ui
                .button(label)
                .on_hover_text(path.display().to_string())
                .clicked()
            {
                state.shared_output_dir = Some(path.clone());
                remember_shared_output_dir(state, path);
                *output_changed = true;
            }
        }
    });
}

fn draw_suggested_output_dirs(ui: &mut egui::Ui, state: &mut AppState, output_changed: &mut bool) {
    let suggestions = suggested_output_dirs(&state.input_paths);
    if suggestions.is_empty() {
        return;
    }

    let current = state.effective_shared_output_dir();
    ui.horizontal_wrapped(|ui| {
        ui.small("Suggested:");
        for path in suggestions {
            let is_current = current.as_ref() == Some(&path);
            let valid_target = !path.exists() || path.is_dir();
            let hover_text = if path.exists() {
                path.display().to_string()
            } else {
                format!("Will be created: {}", path.display())
            };

            if ui
                .add_enabled(
                    valid_target && !is_current,
                    egui::Button::new(path_button_label(&path)).small(),
                )
                .on_hover_text(hover_text)
                .clicked()
            {
                remember_shared_output_dir(state, path.clone());
                state.shared_output_dir = Some(path);
                *output_changed = true;
            }
        }
    });
}

fn suggested_output_dirs(input_paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut suggestions = Vec::new();
    if let Some(default_dir) = image_processing::get_shared_output_dir(input_paths) {
        push_unique_path(&mut suggestions, default_dir);
    }

    for input_path in input_paths {
        push_unique_path(
            &mut suggestions,
            output_dir_suggestion_for_input(input_path.as_path()),
        );
    }

    suggestions
}

fn output_dir_suggestion_for_input(input_path: &Path) -> PathBuf {
    if input_path.is_file()
        && let Some(parent) = input_path.parent()
    {
        return image_processing::get_output_dir(parent);
    }
    image_processing::get_output_dir(input_path)
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn path_button_label(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().to_string(),
    )
}

#[expect(clippy::cast_precision_loss)]
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} bytes")
    }
}

#[expect(clippy::too_many_lines)]
fn draw_processing_actions(ui: &mut egui::Ui, state: &mut AppState) {
    let mut selected_preview_changed = false;

    selected_preview_changed |= ui
        .checkbox(&mut state.image_manipulation_enabled, "Modify image data")
        .changed();

    ui.add_enabled_ui(state.image_manipulation_enabled, |ui| {
        selected_preview_changed |= ui
            .checkbox(&mut state.crop_to_content, "Crop images to content")
            .changed();

        ui.add_enabled_ui(state.crop_to_content, |ui| {
            ui.horizontal(|ui| {
                ui.label("Crop threshold:");
                selected_preview_changed |= ui
                    .add(egui::Slider::new(&mut state.crop_threshold, 0..=255))
                    .changed();
            });

            ui.horizontal(|ui| {
                ui.label("Preview mode:");
                selected_preview_changed |=
                    egui::ComboBox::from_id_salt("studio_binarization_mode")
                        .selected_text(match state.binarization_mode {
                            BinarizationMode::KeepWhite => "Keep White",
                            BinarizationMode::KeepBlack => "Keep Black",
                        })
                        .show_ui(ui, |ui| {
                            let mut changed = false;
                            changed |= ui
                                .selectable_value(
                                    &mut state.binarization_mode,
                                    BinarizationMode::KeepWhite,
                                    "Keep White",
                                )
                                .clicked();
                            changed |= ui
                                .selectable_value(
                                    &mut state.binarization_mode,
                                    BinarizationMode::KeepBlack,
                                    "Keep Black",
                                )
                                .clicked();
                            changed
                        })
                        .inner
                        .unwrap_or(false);
            });

            ui.horizontal(|ui| {
                selected_preview_changed |= ui
                    .checkbox(&mut state.show_crop_bounding_box, "Show crop preview box")
                    .changed();
            });

            ui.horizontal(|ui| {
                ui.label("Box thickness:");
                selected_preview_changed |= ui
                    .add_enabled(
                        state.show_crop_bounding_box,
                        egui::Slider::new(&mut state.box_thickness, 1..=50).text("px"),
                    )
                    .changed();
            });
        });

        ui.horizontal(|ui| {
            ui.label("JPEG quality:");
            selected_preview_changed |= ui
                .add(egui::Slider::new(&mut state.jpeg_quality, 1..=100).text("%"))
                .changed();
        });

        ui.horizontal(|ui| {
            let mut max_size_enabled = state.max_file_size_bytes.is_some();
            let enabled_changed = ui
                .checkbox(&mut max_size_enabled, "Reduce file size")
                .changed();
            if enabled_changed {
                state.max_file_size_bytes = max_size_enabled.then_some(DEFAULT_MAX_FILE_SIZE_BYTES);
                selected_preview_changed = true;
            }

            let mut max_size_kb = state
                .max_file_size_bytes
                .unwrap_or(DEFAULT_MAX_FILE_SIZE_BYTES)
                .div_ceil(1024);
            let size_changed = ui
                .add_enabled(
                    max_size_enabled,
                    egui::DragValue::new(&mut max_size_kb)
                        .range(1..=10 * 1024 * 1024)
                        .speed(10)
                        .suffix(" KB"),
                )
                .changed();
            if max_size_enabled && size_changed {
                state.max_file_size_bytes = Some(max_size_kb.saturating_mul(1024));
                selected_preview_changed = true;
            }
        });

        ui.checkbox(&mut state.sync_preview_pan_zoom, "Sync preview pan/zoom");
    });

    if selected_preview_changed && state.selected_input_file.is_some() {
        state.update_selected_output_info();
    }

    draw_selected_processing_info(ui, state);
}

fn draw_selected_processing_info(ui: &mut egui::Ui, state: &AppState) {
    ui.separator();

    let Some(input_path) = state.selected_input_file.as_ref() else {
        ui.small("Select an image to preview processing effects.");
        return;
    };

    ui.strong("Selected image");
    ui.small(input_path.display().to_string());

    match std::fs::metadata(input_path) {
        Ok(meta) => {
            ui.horizontal(|ui| {
                ui.label("Input size:");
                ui.strong(format_file_size(meta.len()));
            });
        }
        Err(_) => {
            ui.colored_label(egui::Color32::RED, "Could not read input file size.");
        }
    }

    if state.output_info_loading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Calculating output preview...");
        });
    } else if let Some(output_info) = state.selected_output_info.as_ref() {
        ui.horizontal(|ui| {
            ui.label("Output size:");
            ui.strong(format_file_size(output_info.estimated_size));
        });

        ui.horizontal(|ui| {
            ui.label("Dimensions:");
            ui.label(format!(
                "{}x{} -> {}x{}",
                output_info.original_width,
                output_info.original_height,
                output_info.output_width,
                output_info.output_height
            ));
        });

        if output_info.was_cropped {
            ui.small("Crop will change the image dimensions.");
        }
    } else {
        ui.small("Output preview will appear after processing settings are calculated.");
    }
}

#[expect(clippy::cast_possible_truncation)]
fn draw_name_actions(ui: &mut egui::Ui, state: &mut AppState) {
    if ui
        .checkbox(&mut state.rename_rules_enabled, "Apply rename rules")
        .changed()
    {
        state.rename_preview_key = 0;
    }

    ui.add_enabled_ui(state.rename_rules_enabled, |ui| {
        if ui
            .checkbox(&mut state.rename_hyphenate, "Hyphenate camelCase")
            .changed()
        {
            state.rename_preview_key = 0;
        }

        ui.label("Find & replace rules:");
        super::draw_rename_rules_editor(ui, state, "studio_guide_rules_scroll", Some(220.0));
    });

    if ui
        .checkbox(&mut state.max_name_length_enforced, "Warn about long names")
        .changed()
    {
        if let Err(e) = MaxNameLengthEnforced::set_to(&APP_HOME, state.max_name_length_enforced) {
            tracing::error!("Failed to save max-name enforcement: {}", e);
        }
        state.rename_preview_key = 0;
    }

    ui.horizontal(|ui| {
        ui.label("Max name length:");
        let mut value = state.max_name_length as u32;
        if ui
            .add_enabled(
                state.max_name_length_enforced,
                egui::DragValue::new(&mut value).range(10..=500).speed(1.0),
            )
            .changed()
        {
            state.max_name_length = value as usize;
            if let Err(e) = MaxNameLength::set_to(&APP_HOME, state.max_name_length) {
                tracing::error!("Failed to save max name length: {}", e);
            }
            state.rename_preview_key = 0;
        }
        if ui
            .add_enabled(
                state.max_name_length != MaxNameLength::DEFAULT,
                egui::Button::new("Reset"),
            )
            .clicked()
        {
            state.max_name_length = MaxNameLength::DEFAULT;
            if let Err(e) = MaxNameLength::set_to(&APP_HOME, MaxNameLength::DEFAULT) {
                tracing::error!("Failed to reset max name length: {}", e);
            }
            state.rename_preview_key = 0;
        }
    });
}

fn draw_run_actions(
    ui: &mut egui::Ui,
    state: &mut AppState,
    cm_plan: &plan::CmPlan,
    can_execute: bool,
) {
    draw_run_preview(ui, cm_plan);

    ui.horizontal_wrapped(|ui| {
        ui.checkbox(
            &mut state.auto_search_on_process,
            "Auto-search while processing",
        );
        ui.add_enabled(
            state.auto_search_on_process,
            egui::Checkbox::new(&mut state.auto_search_only_if_sku, "Only when SKU is found"),
        );
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button("Export Plan").clicked() {
            if let Err(e) = plan::export_plan(&APP_HOME, cm_plan) {
                tracing::error!("Failed to export CM v2 plan: {}", e);
            } else {
                tracing::info!("Exported CM v2 plan");
            }
        }

        let process_all = egui::Button::new("Process All");
        if state.process_all_running {
            if ui.button("Cancel").clicked() {
                state.cancel_process_all();
            }
            ui.add_enabled(false, process_all);
        } else if ui.add_enabled(can_execute, process_all).clicked() {
            state.process_all();
        }

        let can_process_selected = can_execute && state.selected_input_file.is_some();
        if ui
            .add_enabled(can_process_selected, egui::Button::new("Process Selected"))
            .clicked()
        {
            state.process_selected();
        }
    });

    if state.process_all_running {
        if let Some((current, total)) = state.process_all_progress {
            ui.add(egui::ProgressBar::new(current as f32 / total.max(1) as f32).show_percentage());
            ui.label(format!("Processing {current}/{total}"));
        } else {
            ui.spinner();
        }
    }

    draw_latest_process_result(ui, state);
}

fn draw_latest_process_result(ui: &mut egui::Ui, state: &AppState) {
    let Some(result) = state.last_process_result.as_ref() else {
        return;
    };

    ui.separator();
    if result.error_count == 0 {
        ui.colored_label(
            egui::Color32::LIGHT_GREEN,
            format!("Processed {} file(s) successfully.", result.processed_count),
        );
    } else {
        ui.colored_label(
            egui::Color32::RED,
            format!(
                "Processed {} file(s) with {} error(s).",
                result.processed_count, result.error_count
            ),
        );
        ui.collapsing("Processing errors", |ui| {
            for error in &result.errors {
                ui.label(error);
            }
        });
    }
}

fn draw_run_preview(ui: &mut egui::Ui, plan: &plan::CmPlan) {
    ui.collapsing("Plan preview", |ui| {
        for entry in plan.entries.iter().take(5) {
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
            });
        }
        if plan.entries.len() > 5 {
            ui.small(format!(
                "{} more planned operations are visible in the Plan tile",
                plan.entries.len() - 5
            ));
        }
    });
}

fn add_input_paths(state: &mut AppState, paths: Vec<PathBuf>) {
    match inputs::add_paths(&APP_HOME, &paths) {
        Ok(added) => {
            tracing::info!("Added {} inputs", added.len());
            remember_input_paths(state, &paths);
            state.reload_data();
        }
        Err(e) => tracing::error!("Failed to add input paths: {}", e),
    }
}

fn refresh_selected_paths(state: &mut AppState) {
    if let Some(selected) = state.selected_input_file.clone() {
        state.select_file(&selected);
    } else {
        state.update_rename_preview();
    }
}

fn remember_shared_output_dir(state: &mut AppState, path: PathBuf) {
    match crate::recent_output_dirs::remember(&APP_HOME, &path) {
        Ok(recent) => state.recent_output_dirs = recent,
        Err(e) => tracing::error!("Failed to remember output folder {}: {}", path.display(), e),
    }
}

fn remember_input_paths(state: &mut AppState, paths: &[PathBuf]) {
    match crate::recent_input_paths::remember_many(&APP_HOME, paths) {
        Ok(recent) => state.recent_input_paths = recent,
        Err(e) => tracing::error!("Failed to remember input paths: {}", e),
    }
}

fn decision_value(plan: &plan::CmPlan, name: &str) -> String {
    plan.decisions
        .iter()
        .find(|decision| decision.name == name)
        .map_or_else(
            || "not found".to_string(),
            |decision| decision.value.clone(),
        )
}

fn decision_status(plan: &plan::CmPlan, name: &str) -> DecisionStatus {
    plan.decisions
        .iter()
        .find(|decision| decision.name == name)
        .map_or(DecisionStatus::Invalid, |decision| decision.status.clone())
}

fn merged_status(left: DecisionStatus, right: DecisionStatus) -> DecisionStatus {
    if matches!(left, DecisionStatus::Invalid) || matches!(right, DecisionStatus::Invalid) {
        DecisionStatus::Invalid
    } else if matches!(left, DecisionStatus::Waiting) || matches!(right, DecisionStatus::Waiting) {
        DecisionStatus::Waiting
    } else {
        DecisionStatus::Ready
    }
}

fn status_label(status: &DecisionStatus) -> &'static str {
    match status {
        DecisionStatus::Waiting => "waiting",
        DecisionStatus::Ready => "ready",
        DecisionStatus::Invalid => "invalid",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_dir_suggestions_include_shared_and_input_derived_dirs() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        let file_parent = dir.path().join("loose");
        let file = file_parent.join("photo.jpg");
        std::fs::create_dir_all(&first)?;
        std::fs::create_dir_all(&second)?;
        std::fs::create_dir_all(&file_parent)?;
        std::fs::write(&file, b"not decoded")?;

        let suggestions = suggested_output_dirs(&[first.clone(), second.clone(), file]);

        assert_eq!(
            suggestions,
            vec![
                dir.path().parent().unwrap().join(format!(
                    "{}-output",
                    dir.path().file_name().unwrap().to_string_lossy()
                )),
                dir.path().join("first-output"),
                dir.path().join("second-output"),
                dir.path().join("loose-output"),
            ]
        );
        Ok(())
    }

    #[test]
    fn output_dir_suggestions_dedupe_default_and_single_input_dir() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let input = dir.path().join("photos");
        std::fs::create_dir_all(&input)?;

        let suggestions = suggested_output_dirs(&[input]);

        assert_eq!(suggestions, vec![dir.path().join("photos-output")]);
        Ok(())
    }

    #[test]
    fn file_size_formatter_uses_human_units() {
        assert_eq!(format_file_size(12), "12 bytes");
        assert_eq!(format_file_size(1536), "1.50 KB");
        assert_eq!(format_file_size(2 * 1024 * 1024), "2.00 MB");
    }
}
