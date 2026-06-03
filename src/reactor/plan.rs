//! Inspectable v2 Reactor plan model shared with the egui shell.

use crate::app_home::AppHome;
use crate::egui::state::AppState;
use crate::image_processing;
use crate::image_processing::OutputPathOptions;
use facet::Facet;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum DecisionStatus {
    Waiting,
    Ready,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct DecisionSummary {
    pub name: String,
    pub status: DecisionStatus,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct PlanTransformation {
    pub name: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct PlanEntry {
    pub input_path: PathBuf,
    pub input_root: Option<PathBuf>,
    pub renamed_file_name: Option<String>,
    pub desired_output_path: Option<PathBuf>,
    pub reserved_output_path: Option<PathBuf>,
    pub transformations: Vec<PlanTransformation>,
    pub expectations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct CmPlan {
    pub decisions: Vec<DecisionSummary>,
    pub entries: Vec<PlanEntry>,
    pub errors: Vec<String>,
}

impl CmPlan {
    #[must_use]
    pub fn ready_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.status == DecisionStatus::Ready)
            .count()
    }

    #[must_use]
    pub fn waiting_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.status == DecisionStatus::Waiting)
            .count()
    }

    #[must_use]
    pub fn invalid_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.status == DecisionStatus::Invalid)
            .count()
    }

    #[must_use]
    pub fn collision_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.output_path_was_reserved())
            .count()
    }

    /// # Errors
    ///
    /// Fails if writeln! fails on the output string, which should be infallible since we're writing to a String.
    pub fn render_text(&self) -> eyre::Result<String> {
        let mut out = String::new();
        out.push_str("CM v2 plan\n");
        out.push_str("==========\n\n");
        writeln!(
            out,
            "Decisions: {} ready, {} waiting, {} invalid",
            self.ready_count(),
            self.waiting_count(),
            self.invalid_count()
        )?;
        write!(
            out,
            "Plan entries: {}\nErrors: {}\n\n",
            self.entries.len(),
            self.errors.len()
        )?;

        out.push_str("Decisions\n---------\n");
        for decision in &self.decisions {
            writeln!(
                out,
                "- [{}] {}: {}",
                decision.status.as_str(),
                decision.name,
                decision.value
            )?;
        }

        out.push_str("\nEntries\n-------\n");
        for (idx, entry) in self.entries.iter().enumerate() {
            writeln!(out, "{}. {}", idx + 1, entry.input_path.display())?;
            if let Some(root) = &entry.input_root {
                writeln!(out, "   input root: {}", root.display())?;
            }
            if let Some(name) = &entry.renamed_file_name {
                writeln!(out, "   output name: {name}")?;
            }
            if let Some(path) = &entry.desired_output_path {
                writeln!(out, "   desired output: {}", path.display())?;
            }
            if let Some(path) = &entry.reserved_output_path {
                writeln!(out, "   reserved output: {}", path.display())?;
            }
            for transformation in &entry.transformations {
                writeln!(
                    out,
                    "   transform: {} - {}",
                    transformation.name, transformation.detail
                )?;
            }
            for expectation in &entry.expectations {
                writeln!(out, "   expect: {expectation}")?;
            }
        }

        if !self.errors.is_empty() {
            out.push_str("\nErrors\n------\n");
            for error in &self.errors {
                writeln!(out, "- {error}")?;
            }
        }

        Ok(out)
    }

    /// Render the plan as structured JSON for agent and automation inspection.
    ///
    /// # Errors
    /// Returns an error if the plan cannot be encoded as JSON.
    pub fn render_json(&self) -> eyre::Result<String> {
        facet_json::to_string_pretty(self)
            .map_err(|e| eyre::eyre!("Failed to serialize CM v2 plan as JSON: {}", e))
    }
}

impl PlanEntry {
    #[must_use]
    pub fn output_path_was_reserved(&self) -> bool {
        matches!(
            (&self.desired_output_path, &self.reserved_output_path),
            (Some(desired), Some(reserved)) if desired != reserved
        )
    }
}

impl DecisionStatus {
    fn as_str(self) -> &'static str {
        match self {
            DecisionStatus::Waiting => "waiting",
            DecisionStatus::Ready => "ready",
            DecisionStatus::Invalid => "invalid",
        }
    }
}

#[must_use]
pub fn build_plan(state: &mut AppState) -> CmPlan {
    state.update_rename_preview();

    let output_path_options = state.output_path_options();
    let decisions = build_decisions(state, &output_path_options);
    let (entries, mut errors) = build_entries(state, &output_path_options);
    errors.extend(output_root_errors(state, &output_path_options));

    CmPlan {
        decisions,
        entries,
        errors,
    }
}

/// Export the plan as both readable text and structured JSON under the given app home.
///
/// # Errors
/// Returns an error if the app home cannot be created or either export file cannot be written.
pub fn export_plan(home: &AppHome, cm_plan: &CmPlan) -> eyre::Result<()> {
    home.ensure_dir()?;
    std::fs::write(home.file_path("last-plan.txt"), cm_plan.render_text()?)?;
    std::fs::write(home.file_path("last-plan.json"), cm_plan.render_json()?)?;
    Ok(())
}

fn build_decisions(
    state: &AppState,
    output_path_options: &OutputPathOptions,
) -> Vec<DecisionSummary> {
    vec![
        DecisionSummary {
            name: "input paths".to_string(),
            status: if state.input_paths.is_empty() {
                DecisionStatus::Waiting
            } else {
                DecisionStatus::Ready
            },
            value: format!("{} selected", state.input_paths.len()),
        },
        DecisionSummary {
            name: "input image paths".to_string(),
            status: if state.image_files.is_empty() {
                DecisionStatus::Waiting
            } else {
                DecisionStatus::Ready
            },
            value: format!("{} images discovered", state.image_files.len()),
        },
        DecisionSummary {
            name: "output directory strategy".to_string(),
            status: output_directory_strategy_status(state, output_path_options),
            value: output_strategy_label(state, output_path_options),
        },
        DecisionSummary {
            name: "crop images to content".to_string(),
            status: if state.image_manipulation_enabled {
                DecisionStatus::Ready
            } else {
                DecisionStatus::Waiting
            },
            value: if state.image_manipulation_enabled {
                let box_label = if state.show_crop_bounding_box {
                    format!("box {} px", state.box_thickness)
                } else {
                    "box hidden".to_string()
                };
                format!(
                    "{} at threshold {}, {}",
                    bool_label(state.crop_to_content),
                    state.crop_threshold,
                    box_label
                )
            } else {
                "image bytes preserved".to_string()
            },
        },
        DecisionSummary {
            name: "reduce file size".to_string(),
            status: if state.image_manipulation_enabled {
                DecisionStatus::Ready
            } else {
                DecisionStatus::Waiting
            },
            value: state.max_file_size_bytes.map_or_else(
                || "disabled".to_string(),
                |bytes| format!("enabled with limit {}", format_size(bytes)),
            ),
        },
        DecisionSummary {
            name: "rename images".to_string(),
            status: DecisionStatus::Ready,
            value: format!(
                "{} rules, hyphenate camelCase {}",
                state.rename_rules.len(),
                bool_label(state.rename_hyphenate)
            ),
        },
        DecisionSummary {
            name: "max file name length".to_string(),
            status: DecisionStatus::Ready,
            value: if state.max_name_length_enforced {
                format!("enforced at {} characters", state.max_name_length)
            } else {
                "not enforced".to_string()
            },
        },
        DecisionSummary {
            name: "selected input image".to_string(),
            status: if state.selected_input_file.is_some() {
                DecisionStatus::Ready
            } else {
                DecisionStatus::Waiting
            },
            value: state
                .selected_input_file
                .as_ref()
                .map_or_else(|| "none".to_string(), |path| path.display().to_string()),
        },
    ]
}

fn output_directory_strategy_status(
    state: &AppState,
    output_path_options: &OutputPathOptions,
) -> DecisionStatus {
    if state.save_all_inputs_to_same_folder
        && output_path_options.shared_output_dir.is_none()
        && image_processing::get_shared_output_dir(&state.input_paths).is_none()
    {
        DecisionStatus::Waiting
    } else if planned_output_roots(state, output_path_options)
        .iter()
        .any(|path| path.exists() && !path.is_dir())
    {
        DecisionStatus::Invalid
    } else {
        DecisionStatus::Ready
    }
}

fn output_root_errors(state: &AppState, output_path_options: &OutputPathOptions) -> Vec<String> {
    planned_output_roots(state, output_path_options)
        .into_iter()
        .filter(|path| path.exists() && !path.is_dir())
        .map(|path| format!("Output folder path is not a directory: {}", path.display()))
        .collect()
}

fn planned_output_roots(state: &AppState, output_path_options: &OutputPathOptions) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if state.save_all_inputs_to_same_folder {
        if let Some(path) = output_path_options
            .shared_output_dir
            .clone()
            .or_else(|| image_processing::get_shared_output_dir(&state.input_paths))
        {
            push_unique_path(&mut roots, path);
        }
    } else {
        for input_path in &state.input_paths {
            push_unique_path(&mut roots, image_processing::get_output_dir(input_path));
        }
    }
    roots
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn build_entries(
    state: &AppState,
    output_path_options: &OutputPathOptions,
) -> (Vec<PlanEntry>, Vec<String>) {
    let reserved_output_paths = Mutex::new(HashSet::new());
    let mut entries = Vec::new();
    let mut errors = Vec::new();

    for (idx, input_path) in state.image_files.iter().enumerate() {
        let mut expectations = Vec::new();
        let input_root = state
            .input_paths
            .iter()
            .find(|root| input_path.starts_with(root))
            .cloned();

        if input_path.exists() {
            expectations.push("input path exists".to_string());
        } else {
            expectations.push("input path is missing".to_string());
            errors.push(format!("Input path is missing: {}", input_path.display()));
        }

        let renamed_file_name = state.renamed_files.get(idx).and_then(|renamed| {
            renamed
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        });

        let desired_output_path = input_root.as_ref().and_then(|root| {
            renamed_file_name.as_ref().and_then(|name| {
                image_processing::get_output_path_with_options(
                    input_path,
                    root,
                    name,
                    &state.input_paths,
                    output_path_options,
                )
            })
        });

        let reserved_output_path = desired_output_path.as_ref().map(|path| {
            image_processing::reserve_available_output_path(path, &reserved_output_paths)
        });
        let transformations = build_transformations(
            state,
            input_path,
            renamed_file_name.as_deref(),
            desired_output_path.as_ref(),
            reserved_output_path.as_ref(),
        );

        if input_root.is_none() {
            errors.push(format!(
                "Could not find input root for {}",
                input_path.display()
            ));
        }
        if renamed_file_name.is_none() {
            errors.push(format!(
                "Could not determine renamed file name for {}",
                input_path.display()
            ));
        }
        if desired_output_path.is_none() {
            errors.push(format!(
                "Could not calculate output path for {}",
                input_path.display()
            ));
        }

        populate_output_path_details(
            input_path,
            desired_output_path.as_ref(),
            reserved_output_path.as_ref(),
            &mut expectations,
            &mut errors,
        );

        entries.push(PlanEntry {
            input_path: input_path.clone(),
            input_root,
            renamed_file_name,
            desired_output_path,
            reserved_output_path,
            transformations,
            expectations,
        });
    }

    (entries, errors)
}

fn populate_output_path_details(
    input_path: &Path,
    desired_output_path: Option<&PathBuf>,
    reserved_output_path: Option<&PathBuf>,
    expectations: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    let Some(path) = reserved_output_path else {
        return;
    };

    if desired_output_path.is_some_and(|desired| desired.as_path() == input_path) {
        errors.push(format!(
            "Output would overwrite input: {}",
            input_path.display()
        ));
    } else if path.as_path() == input_path {
        errors.push(format!(
            "Reserved output would overwrite input: {}",
            input_path.display()
        ));
    } else {
        expectations.push("output path does not overwrite input path".to_string());
    }

    if let Some(desired) = desired_output_path
        && desired != path
    {
        expectations.push(format!(
            "output path collision handled: {} -> {}",
            desired.display(),
            path.display()
        ));
    }

    if let Some(parent) = path.parent() {
        expectations.push(format!(
            "output parent can be created: {}",
            parent.display()
        ));
    }
}

fn build_transformations(
    state: &AppState,
    input_path: &Path,
    renamed_file_name: Option<&str>,
    desired_output_path: Option<&PathBuf>,
    reserved_output_path: Option<&PathBuf>,
) -> Vec<PlanTransformation> {
    let mut transformations = Vec::new();

    transformations.push(PlanTransformation {
        name: "read".to_string(),
        detail: format!("read image bytes from {}", input_path.display()),
    });

    transformations.push(rename_transformation(state, input_path, renamed_file_name));

    transformations.push(image_processing_transformation(state));

    transformations.push(metadata_transformation(state));

    transformations.push(PlanTransformation {
        name: "reserve output path".to_string(),
        detail: match (desired_output_path, reserved_output_path) {
            (Some(desired), Some(reserved)) if desired == reserved => {
                format!("use {}", reserved.display())
            }
            (Some(desired), Some(reserved)) => {
                format!("{} resolves to {}", desired.display(), reserved.display())
            }
            _ => "waiting for output path".to_string(),
        },
    });

    transformations.push(PlanTransformation {
        name: "write".to_string(),
        detail: reserved_output_path.map_or_else(
            || "waiting for output path".to_string(),
            |path| format!("write processed bytes to {}", path.display()),
        ),
    });

    transformations
}

fn rename_transformation(
    state: &AppState,
    input_path: &Path,
    renamed_file_name: Option<&str>,
) -> PlanTransformation {
    let original_name = input_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    let detail = if !state.rename_rules_enabled {
        "rename rules disabled".to_string()
    } else if let Some(renamed) = renamed_file_name {
        if renamed == original_name {
            "keep original file name".to_string()
        } else {
            format!("{original_name} -> {renamed}")
        }
    } else {
        "waiting for renamed file name".to_string()
    };

    PlanTransformation {
        name: "rename".to_string(),
        detail,
    }
}

fn image_processing_transformation(state: &AppState) -> PlanTransformation {
    let detail = if state.image_manipulation_enabled {
        let crop = if state.crop_to_content {
            let box_label = if state.show_crop_bounding_box {
                format!("show preview box {} px", state.box_thickness)
            } else {
                "hide preview box".to_string()
            };
            format!(
                "crop to content at threshold {}; {}",
                state.crop_threshold, box_label
            )
        } else {
            "do not crop".to_string()
        };
        let size = state.max_file_size_bytes.map_or_else(
            || "file-size reduction disabled".to_string(),
            |bytes| format!("reduce file size to {}", format_size(bytes)),
        );
        format!("{crop}; JPEG quality {}; {size}", state.jpeg_quality)
    } else {
        "preserve original image bytes".to_string()
    };

    PlanTransformation {
        name: "image processing".to_string(),
        detail,
    }
}

fn metadata_transformation(state: &AppState) -> PlanTransformation {
    let detail = if state.auto_search_on_process {
        if state.auto_search_only_if_sku {
            "search selected product only when filename contains a SKU and write description metadata"
                .to_string()
        } else {
            "search selected product and write description metadata".to_string()
        }
    } else {
        "do not add product-search description metadata".to_string()
    };

    PlanTransformation {
        name: "metadata".to_string(),
        detail,
    }
}

fn output_strategy_label(state: &AppState, output_path_options: &OutputPathOptions) -> String {
    if state.save_all_inputs_to_same_folder {
        output_path_options.shared_output_dir.as_ref().map_or_else(
            || "shared output folder pending".to_string(),
            |path| format!("shared folder {}", path.display()),
        )
    } else {
        "one -output folder per input root".to_string()
    }
}

fn bool_label(value: bool) -> &'static str {
    if value { "enabled" } else { "disabled" }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_plan_waits_for_inputs() {
        let mut state = AppState::default();

        let plan = build_plan(&mut state);

        assert_eq!(plan.entries.len(), 0);
        assert!(plan.waiting_count() > 0);
        assert!(plan.render_text().unwrap().contains("CM v2 plan"));
    }

    #[test]
    fn plan_entry_names_transformation_pipeline() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let input_dir = dir.path().join("input");
        std::fs::create_dir_all(&input_dir)?;
        let input_file = input_dir.join("PhotoPack.jpg");
        std::fs::write(&input_file, b"not actually decoded here")?;

        let mut state = AppState::default();
        state.input_paths = vec![input_dir];
        state.image_files = vec![input_file];
        state.rename_rules = vec![crate::rename_rules::RenameRule {
            find: "Pack".to_string(),
            replace: "".to_string(),
            only_when_name_too_long: false,
            ..crate::rename_rules::RenameRule::default()
        }];
        state.auto_search_on_process = true;

        let plan = build_plan(&mut state);

        let entry = plan.entries.first().expect("one plan entry");
        let names: Vec<_> = entry
            .transformations
            .iter()
            .map(|transformation| transformation.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "read",
                "rename",
                "image processing",
                "metadata",
                "reserve output path",
                "write"
            ]
        );
        assert!(plan.render_text().unwrap().contains("transform: rename"));
        Ok(())
    }

    #[test]
    fn disabled_max_name_length_skips_too_long_rename_rules() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let input_dir = dir.path().join("input");
        std::fs::create_dir_all(&input_dir)?;
        let input_file = input_dir.join("VeryLongPhotoName.jpg");
        std::fs::write(&input_file, b"not actually decoded here")?;

        let mut state = AppState::default();
        state.input_paths = vec![input_dir];
        state.image_files = vec![input_file];
        state.max_name_length = 5;
        state.max_name_length_enforced = false;
        state.rename_rules = vec![crate::rename_rules::RenameRule {
            find: "VeryLong".to_string(),
            replace: "Short".to_string(),
            only_when_name_too_long: true,
            ..crate::rename_rules::RenameRule::default()
        }];

        let plan = build_plan(&mut state);

        let entry = plan.entries.first().expect("one plan entry");
        assert_eq!(
            entry.renamed_file_name.as_deref(),
            Some("VeryLongPhotoName.jpg")
        );
        assert!(
            plan.render_text()
                .unwrap()
                .contains("max file name length: not enforced")
        );
        Ok(())
    }

    #[test]
    fn plan_renders_structured_json() {
        let mut state = AppState::default();

        let plan = build_plan(&mut state);
        let json = plan.render_json().expect("plan json");

        assert!(json.contains("\"decisions\""));
        assert!(json.contains("\"entries\""));
        assert!(json.contains("\"errors\""));
    }

    #[test]
    fn reduce_file_size_decision_names_enabled_and_disabled_states() {
        let mut state = AppState::default();

        let plan = build_plan(&mut state);
        let decision = plan
            .decisions
            .iter()
            .find(|decision| decision.name == "reduce file size")
            .expect("reduce file size decision");
        assert_eq!(decision.value, "disabled");

        state.max_file_size_bytes = Some(crate::image_processing::DEFAULT_MAX_FILE_SIZE_BYTES);
        let plan = build_plan(&mut state);
        let decision = plan
            .decisions
            .iter()
            .find(|decision| decision.name == "reduce file size")
            .expect("reduce file size decision");
        assert_eq!(decision.value, "enabled with limit 50 MB");
    }

    #[test]
    fn export_plan_writes_text_and_json_artifacts() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let home = AppHome(dir.path().to_path_buf());
        let mut state = AppState::default();
        let plan = build_plan(&mut state);

        export_plan(&home, &plan)?;

        assert!(home.file_path("last-plan.txt").exists());
        assert!(home.file_path("last-plan.json").exists());
        assert!(std::fs::read_to_string(home.file_path("last-plan.txt"))?.contains("CM v2 plan"));
        assert!(
            std::fs::read_to_string(home.file_path("last-plan.json"))?.contains("\"decisions\"")
        );
        Ok(())
    }

    #[test]
    fn shared_output_file_marks_output_strategy_invalid() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let output_file = dir.path().join("not-a-folder");
        std::fs::write(&output_file, b"existing file")?;

        let mut state = AppState::default();
        state.save_all_inputs_to_same_folder = true;
        state.shared_output_dir = Some(output_file.clone());

        let plan = build_plan(&mut state);

        let decision = plan
            .decisions
            .iter()
            .find(|decision| decision.name == "output directory strategy")
            .expect("output strategy decision");
        assert_eq!(decision.status, DecisionStatus::Invalid);
        assert!(
            plan.errors
                .iter()
                .any(|error| error.contains("not-a-folder"))
        );
        Ok(())
    }

    #[test]
    fn per_input_output_file_marks_output_strategy_invalid() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let input_dir = dir.path().join("photos");
        let output_file = dir.path().join("photos-output");
        std::fs::create_dir_all(&input_dir)?;
        std::fs::write(&output_file, b"existing file")?;

        let mut state = AppState::default();
        state.input_paths = vec![input_dir];

        let plan = build_plan(&mut state);

        let decision = plan
            .decisions
            .iter()
            .find(|decision| decision.name == "output directory strategy")
            .expect("output strategy decision");
        assert_eq!(decision.status, DecisionStatus::Invalid);
        assert!(
            plan.errors
                .iter()
                .any(|error| error.contains("photos-output"))
        );
        Ok(())
    }

    #[test]
    fn duplicate_desired_outputs_are_visible_as_reserved_collisions() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let input_dir = dir.path().join("photos");
        let left = input_dir.join("left").join("photo.jpg");
        let right = input_dir.join("right").join("photo.jpg");
        std::fs::create_dir_all(left.parent().unwrap())?;
        std::fs::create_dir_all(right.parent().unwrap())?;
        std::fs::write(&left, b"left")?;
        std::fs::write(&right, b"right")?;

        let mut state = AppState::default();
        state.input_paths = vec![input_dir];
        state.image_files = vec![left, right];
        state.flatten_output_hierarchy = true;

        let plan = build_plan(&mut state);

        assert_eq!(plan.collision_count(), 1);
        assert!(plan.entries.iter().any(|entry| {
            entry
                .expectations
                .iter()
                .any(|expectation| expectation.contains("output path collision handled"))
        }));
        Ok(())
    }

    #[test]
    fn output_equal_to_input_is_reported_before_reservation() -> eyre::Result<()> {
        let dir = tempfile::tempdir()?;
        let input_file = dir.path().join("photo.jpg");
        std::fs::write(&input_file, b"input")?;

        let mut state = AppState::default();
        state.input_paths = vec![input_file.clone()];
        state.image_files = vec![input_file];
        state.save_all_inputs_to_same_folder = true;
        state.shared_output_dir = Some(dir.path().to_path_buf());

        let plan = build_plan(&mut state);

        assert_eq!(plan.collision_count(), 1);
        assert!(
            plan.errors
                .iter()
                .any(|error| error.contains("Output would overwrite input"))
        );
        Ok(())
    }
}
