use eyre::eyre;
use tracing::info;

/// Run the GUI. This is async so the caller can create a runtime; the function will
/// block in place on the eframe app using `tokio::task::block_in_place`.
pub async fn run_gui() -> eyre::Result<()> {
    info!("Starting CM GUI");
    let native_options = eframe::NativeOptions::default();

    let res = tokio::task::block_in_place(move || {
        eframe::run_native(
            "CM - Creative Memories Photo Manager",
            native_options,
            Box::new(|cc| Ok(Box::new(CmApp::new(cc)))),
        )
        .map_err(|e| eyre!("Failed to run eframe: {}", e))
    });

    res?;
    info!("GUI exited");
    Ok(())
}

use crate::app_home::APP_HOME;
use crate::inputs;
use crate::rename_rules::RenameRule;
use eframe::egui::Align2;
use eframe::egui::Color32;
use eframe::egui::Id;
use eframe::egui::LayerId;
use eframe::egui::Order;
use eframe::egui::ScrollArea;
use eframe::egui::TextStyle;
use eframe::egui::{self};
use egui_snarl::InPin;
use egui_snarl::NodeId;
use egui_snarl::OutPin;
use egui_snarl::Snarl;
use egui_snarl::ui::PinInfo;
use egui_snarl::ui::SnarlStyle;
use egui_snarl::ui::SnarlViewer;
use egui_snarl::ui::SnarlWidget;
use std::collections::HashMap;
use std::path::PathBuf;

/// Color for path-type pins
const PATH_COLOR: Color32 = Color32::from_rgb(0x00, 0x80, 0xb0);

/// Our node types for the pipeline
#[derive(Clone, Default)]
enum CmNode {
    /// The inputs node - shows persisted input paths
    #[default]
    Inputs,
    /// Identifies image paths from input directories
    ImagePaths,
    /// Rename files using find/replace rules
    RenameFiles {
        preview_key: u64,
        preview: Vec<PathBuf>,
    },
}

/// Viewer for our node graph
struct CmViewer<'a> {
    /// Cached input paths (refreshed each frame from disk)
    input_paths: &'a mut Vec<PathBuf>,
    /// Cached image files (derived from inputs)
    image_files: &'a mut Vec<PathBuf>,
    /// Path to remove (deferred action)
    path_to_remove: &'a mut Option<PathBuf>,
    /// Whether to clear all inputs (deferred action)
    clear_all: &'a mut bool,
    /// Last error message
    last_error: &'a mut Option<String>,
}

impl SnarlViewer<CmNode> for CmViewer<'_> {
    fn title(&mut self, node: &CmNode) -> String {
        match node {
            CmNode::Inputs => format!("Inputs ({} paths)", self.input_paths.len()),
            CmNode::ImagePaths => format!("Image Paths ({} images)", self.image_files.len()),
            CmNode::RenameFiles { .. } => {
                // Show count of global rules
                match crate::rename_rules::list_rules(&crate::app_home::APP_HOME) {
                    Ok(v) => format!("Rename Files ({} rules)", v.len()),
                    Err(_) => "Rename Files".to_string(),
                }
            }
        }
    }

    fn inputs(&mut self, node: &CmNode) -> usize {
        match node {
            CmNode::Inputs => 0,
            CmNode::ImagePaths => 1,
            CmNode::RenameFiles { .. } => 1,
        }
    }

    fn outputs(&mut self, node: &CmNode) -> usize {
        match node {
            CmNode::Inputs => 1,
            CmNode::ImagePaths => 1,
            CmNode::RenameFiles { .. } => 1,
        }
    }

    #[allow(refining_impl_trait)]
    fn show_input(&mut self, pin: &InPin, ui: &mut egui::Ui, snarl: &mut Snarl<CmNode>) -> PinInfo {
        match &snarl[pin.id.node] {
            CmNode::Inputs => {
                unreachable!("Inputs node has no inputs")
            }
            CmNode::ImagePaths => {
                // Input pin for receiving paths
                ui.label("Paths");
                PinInfo::circle().with_fill(PATH_COLOR)
            }
            CmNode::RenameFiles { .. } => {
                // Input pin for receiving paths
                ui.label("Images");
                PinInfo::circle().with_fill(PATH_COLOR)
            }
        }
    }

    #[allow(refining_impl_trait)]
    fn show_output(
        &mut self,
        pin: &OutPin,
        _ui: &mut egui::Ui,
        snarl: &mut Snarl<CmNode>,
    ) -> PinInfo {
        match &snarl[pin.id.node] {
            CmNode::Inputs => {
                // Output pin - just show the connector
                PinInfo::circle().with_fill(PATH_COLOR)
            }
            CmNode::ImagePaths => {
                // Output pin - just show the connector
                PinInfo::circle().with_fill(PATH_COLOR)
            }
            CmNode::RenameFiles { .. } => {
                // Output pin - just show the connector
                PinInfo::circle().with_fill(PATH_COLOR)
            }
        }
    }

    fn has_body(&mut self, _node: &CmNode) -> bool {
        true
    }

    fn show_body(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        snarl: &mut Snarl<CmNode>,
    ) {
        match &snarl[node] {
            CmNode::Inputs => {
                self.show_inputs_body(ui);
            }
            CmNode::ImagePaths => {
                self.show_image_paths_body(ui, node, snarl);
            }
            CmNode::RenameFiles { .. } => {
                self.show_rename_files_body(ui, node, snarl);
            }
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<CmNode>) {
        // Allow valid connections between compatible nodes
        let valid = matches!(
            (&snarl[from.id.node], &snarl[to.id.node]),
            (CmNode::Inputs, CmNode::ImagePaths)
                | (CmNode::ImagePaths, CmNode::RenameFiles { .. })
                | (CmNode::RenameFiles { .. }, CmNode::RenameFiles { .. })
        );

        if valid {
            // Disconnect any existing connections to this input
            for &remote in &to.remotes {
                snarl.disconnect(remote, to.id);
            }
            snarl.connect(from.id, to.id);
        }
    }

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<CmNode>) -> bool {
        true
    }

    fn show_graph_menu(&mut self, pos: egui::Pos2, ui: &mut egui::Ui, snarl: &mut Snarl<CmNode>) {
        ui.label("Add node");
        if ui.button("Inputs").clicked() {
            snarl.insert_node(pos, CmNode::Inputs);
            ui.close();
        }
        if ui.button("Image Paths").clicked() {
            snarl.insert_node(pos, CmNode::ImagePaths);
            ui.close();
        }
        if ui.button("Rename Files").clicked() {
            snarl.insert_node(
                pos,
                CmNode::RenameFiles {
                    preview_key: 0,
                    preview: Vec::new(),
                },
            );
            ui.close();
        }
    }

    fn has_node_menu(&mut self, _node: &CmNode) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        snarl: &mut Snarl<CmNode>,
    ) {
        ui.label("Node menu");
        if ui.button("Remove").clicked() {
            snarl.remove_node(node);
            ui.close();
        }
    }
}

impl CmViewer<'_> {
    fn show_inputs_body(&mut self, ui: &mut egui::Ui) {
        // Show error if any
        if let Some(err) = self.last_error.as_ref() {
            ui.colored_label(Color32::RED, err);
        }

        // Clear all button
        ui.horizontal(|ui| {
            if ui.button("🗑 Clear All").clicked() {
                *self.clear_all = true;
            }
            ui.label("Drag & drop folders here");
        });

        ui.separator();

        if self.input_paths.is_empty() {
            ui.label("(no inputs - drag folders onto the window)");
            return;
        }

        // Show paths with remove buttons
        ScrollArea::vertical()
            .id_salt("inputs_scroll")
            .max_height(300.0)
            .show(ui, |ui| {
                for path in self.input_paths.iter() {
                    ui.horizontal(|ui| {
                        // Remove button
                        if ui.small_button("✖").clicked() {
                            *self.path_to_remove = Some(path.clone());
                        }
                        // Path label (truncate if needed)
                        let display = path.display().to_string();
                        ui.label(&display);
                    });
                }
            });
    }

    fn show_image_paths_body(&mut self, ui: &mut egui::Ui, node_id: NodeId, snarl: &Snarl<CmNode>) {
        // Check if input pin has connections
        let in_pin = snarl.in_pin(egui_snarl::InPinId {
            node: node_id,
            input: 0,
        });
        if in_pin.remotes.is_empty() {
            ui.colored_label(Color32::YELLOW, "(no input connected)");
            ui.add_space(6.0);
            ui.label("Connect the Inputs node to this node to see image paths.");
            return;
        }

        // Use node_id for stable resize widget ID
        egui::Resize::default()
            .id_salt(node_id)
            .default_size(egui::vec2(350.0, 400.0))
            .min_size(egui::vec2(200.0, 100.0))
            .show(ui, |ui| {
                if self.image_files.is_empty() {
                    ui.label("(no image files found)");
                    return;
                }

                // Build a tree structure grouped by input directories
                let grouped =
                    group_files_by_input(self.input_paths.as_slice(), self.image_files.as_slice());

                // Use available size so ScrollArea fills the Resize container
                let available = ui.available_size();
                ScrollArea::both()
                    .id_salt("images_scroll")
                    .auto_shrink([false, false])
                    .max_height(available.y)
                    .max_width(available.x)
                    .show(ui, |ui| {
                        for (input_path, relative_files) in &grouped {
                            show_input_group(ui, input_path, relative_files);
                        }
                    });
            });
    }

    fn show_rename_files_body(
        &mut self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        snarl: &mut Snarl<CmNode>,
    ) {
        use crate::rename_rules::RenameRuleModifier;
        use crate::rename_rules::WhenExpr;

        let in_pin = snarl.in_pin(egui_snarl::InPinId {
            node: node_id,
            input: 0,
        });
        let connected = !in_pin.remotes.is_empty();

        if !matches!(&snarl[node_id], CmNode::RenameFiles { .. }) {
            return;
        }

        ui.vertical(|ui| {
            ui.label("Find & Replace Rules:");
            // ui.separator();

                let mut listed =
                    crate::rename_rules::list_rules(&crate::app_home::APP_HOME).unwrap_or_default();

                for (_, rule) in &mut listed {
                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui.small_button("✖").clicked() {
                                let _ = crate::rename_rules::remove_rule(
                                    &crate::app_home::APP_HOME,
                                    rule.id,
                                );
                                if let CmNode::RenameFiles { preview_key, .. } = &mut snarl[node_id] {
                                    *preview_key = 0;
                                }
                            }

                            ui.label("Find:");
                            ui.add(egui::TextEdit::singleline(&mut rule.find));
                            ui.label("Replace:");
                            ui.add(egui::TextEdit::singleline(&mut rule.replace));

                            if ui.button("Save").clicked() {
                                let _ =
                                    crate::rename_rules::write_rule(&crate::app_home::APP_HOME, rule);
                                if let CmNode::RenameFiles { preview_key, .. } = &mut snarl[node_id] {
                                    *preview_key = 0;
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            let mut ci =
                                rule.modifiers.contains(&RenameRuleModifier::CaseInsensitive);
                            if ui.checkbox(&mut ci, "ci").changed() {
                                if ci {
                                    rule.modifiers.push(RenameRuleModifier::CaseInsensitive);
                                } else {
                                    rule.modifiers
                                        .retain(|m| *m != RenameRuleModifier::CaseInsensitive);
                                }
                            }

                            let mut always = rule.modifiers.contains(&RenameRuleModifier::Always);
                            if ui.checkbox(&mut always, "always").changed() {
                                if always {
                                    rule.modifiers.push(RenameRuleModifier::Always);
                                    rule.modifiers
                                        .retain(|m| !matches!(m, RenameRuleModifier::When(_)));
                                } else {
                                    rule.modifiers.retain(|m| *m != RenameRuleModifier::Always);
                                    rule.modifiers.push(RenameRuleModifier::When(
                                        WhenExpr::LengthIsGreaterThan(50),
                                    ));
                                }
                            }

                            if !always {
                                let len_val = rule
                                    .modifiers
                                    .iter()
                                    .find_map(|m| {
                                        if let RenameRuleModifier::When(
                                            WhenExpr::LengthIsGreaterThan(n),
                                        ) = m
                                        {
                                            Some(*n)
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(50);

                                let mut v = len_val as u32;
                                ui.label("len >");
                                if ui
                                    .add(egui::DragValue::new(&mut v).range(1..=1000))
                                    .changed()
                                {
                                    rule.modifiers
                                        .retain(|m| !matches!(m, RenameRuleModifier::When(_)));
                                    rule.modifiers.push(RenameRuleModifier::When(
                                        WhenExpr::LengthIsGreaterThan(v as usize),
                                    ));
                                }
                            }
                        });
                    });
                }

            if ui.button("+ Add Rule").clicked() {
                let _ = crate::rename_rules::add_rule(
                    &crate::app_home::APP_HOME,
                    &crate::rename_rules::RenameRule::default(),
                );
                if let CmNode::RenameFiles { preview_key, .. } = &mut snarl[node_id] {
                    *preview_key = 0;
                }
            }

            if !connected {
                ui.colored_label(Color32::YELLOW, "(no input connected — preview hidden)");
                ui.add_space(6.0);
                ui.label("Connect an upstream node (e.g., Image Paths) to preview renamed files.");
            } else {
                egui::Resize::default()
                    .id_salt(node_id)
                    .default_size(egui::vec2(350.0, 400.0))
                    .min_size(egui::vec2(200.0, 100.0))
                    .show(ui, |ui| {
                        let available = ui.available_size();
                        ScrollArea::both()
                            .id_salt("renames_preview_scroll")
                            .auto_shrink([false, false])
                            .max_height(available.y)
                            .max_width(available.x)
                            .show(ui, |ui| {
                                let global_rules =
                                    crate::rename_rules::list_rules(&crate::app_home::APP_HOME)
                                        .map(|v| v.into_iter().map(|(_, r)| r).collect::<Vec<_>>())
                                        .unwrap_or_default();

                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::Hash;
                                use std::hash::Hasher;
                                let mut hasher = DefaultHasher::new();
                                self.image_files.len().hash(&mut hasher);
                                for r in &global_rules {
                                    r.id.hash(&mut hasher);
                                    r.find.hash(&mut hasher);
                                    r.replace.hash(&mut hasher);
                                    for m in &r.modifiers {
                                        m.hash(&mut hasher);
                                    }
                                }
                                let key = hasher.finish();

                                if let CmNode::RenameFiles {
                                    preview_key,
                                    preview,
                                } = &mut snarl[node_id]
                                {
                                    if *preview_key != key {
                                        *preview = apply_rules_seq_compiled(
                                            self.image_files.as_slice(),
                                            &global_rules,
                                        );
                                        *preview_key = key;
                                    }

                                    let grouped = group_files_with_renames(
                                        self.input_paths.as_slice(),
                                        self.image_files.as_slice(),
                                        preview,
                                    );

                                    for (input_path, files_with_status) in &grouped {
                                        show_rename_group(ui, input_path, files_with_status);
                                    }
                                }
                            });
                    });
            }
        });
    }
}

/// A simple tree node for displaying paths hierarchically
#[derive(Default)]
struct TreeNode {
    children: HashMap<String, TreeNode>,
    is_file: bool,
}

/// Group image files by which input directory they belong to.
/// Returns a list of (input_path, relative_file_paths) tuples.
fn group_files_by_input(
    input_paths: &[PathBuf],
    image_files: &[PathBuf],
) -> Vec<(PathBuf, Vec<PathBuf>)> {
    let mut result: Vec<(PathBuf, Vec<PathBuf>)> = Vec::new();

    for input_path in input_paths {
        let mut relative_files = Vec::new();

        for file in image_files {
            // Check if this file is under this input path
            if let Ok(relative) = file.strip_prefix(input_path) {
                relative_files.push(relative.to_path_buf());
            }
        }

        if !relative_files.is_empty() {
            relative_files.sort();
            result.push((input_path.clone(), relative_files));
        }
    }

    result
}

/// Build a tree from relative paths
fn build_path_tree(paths: &[PathBuf]) -> TreeNode {
    let mut root = TreeNode::default();

    for path in paths {
        let mut current = &mut root;
        for component in path.components() {
            let name = component.as_os_str().to_string_lossy().to_string();
            current = current.children.entry(name).or_default();
        }
        current.is_file = true;
    }

    root
}

/// Show a group of files under an input directory
fn show_input_group(ui: &mut egui::Ui, input_path: &std::path::Path, relative_files: &[PathBuf]) {
    // Get the display name (last component of the input path)
    let display_name = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.display().to_string());

    // Get the parent path for the tooltip
    let parent_path = input_path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let header_text = format!("📁 {} ({} images)", display_name, relative_files.len());

    let header = egui::CollapsingHeader::new(header_text).default_open(true);

    // Add tooltip with full parent path
    let response = header.show(ui, |ui| {
        let tree = build_path_tree(relative_files);
        show_tree_children(ui, &tree, 0);
    });

    // Show tooltip on the header
    if !parent_path.is_empty() {
        response.header_response.on_hover_text(&parent_path);
    }
}

/// Show tree children (skipping the root level)
fn show_tree_children(ui: &mut egui::Ui, node: &TreeNode, depth: usize) {
    let mut sorted_children: Vec<_> = node.children.iter().collect();
    sorted_children.sort_by_key(|(k, _)| *k);

    for (child_name, child_node) in sorted_children {
        show_tree_node(ui, child_name, child_node, depth);
    }
}

fn show_tree_node(ui: &mut egui::Ui, name: &str, node: &TreeNode, depth: usize) {
    if node.children.is_empty() {
        // Leaf node (file)
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            ui.colored_label(Color32::LIGHT_GREEN, format!("🖼 {name}"));
        });
    } else {
        // Directory with children
        let header_text = format!("📁 {name}");

        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            egui::CollapsingHeader::new(header_text)
                .default_open(depth < 2)
                .show(ui, |ui| {
                    show_tree_children(ui, node, depth + 1);
                });
        });
    }
}

/// Check if a path is an image file
fn is_image_file(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff"
        )
    } else {
        false
    }
}

/// Apply rename rules (regex-based) sequentially to file base names
fn apply_rules_seq_compiled(
    files: &[PathBuf],
    rules: &[crate::rename_rules::RenameRule],
) -> Vec<PathBuf> {
    // Precompile regexes once per rule
    let compiled: Vec<Option<regex::Regex>> = rules
        .iter()
        .map(|r| {
            let mut builder = regex::RegexBuilder::new(&r.find);
            if r.modifiers
                .contains(&crate::rename_rules::RenameRuleModifier::CaseInsensitive)
            {
                builder.case_insensitive(true);
            }
            match builder.build() {
                Ok(re) => Some(re),
                Err(_) => None,
            }
        })
        .collect();

    files
        .iter()
        .map(|path| {
            let original = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let mut cur = original.clone();
            for (i, rule) in rules.iter().enumerate() {
                // Evaluate When modifiers
                let mut skip = false;
                for m in &rule.modifiers {
                    if let crate::rename_rules::RenameRuleModifier::When(
                        crate::rename_rules::WhenExpr::LengthIsGreaterThan(n),
                    ) = m
                    {
                        if cur.len() <= *n {
                            skip = true;
                            break;
                        }
                    }
                }
                if skip {
                    continue;
                }

                if let Some(re) = &compiled[i] {
                    let replaced = re.replace_all(&cur, &rule.replace).to_string();
                    if replaced != cur {
                        cur = replaced;
                    }
                }
            }

            if let Some(parent) = path.parent() {
                parent.join(cur)
            } else {
                PathBuf::from(cur)
            }
        })
        .collect()
}

/// Info about a file and whether it was renamed
struct FileRenameInfo {
    /// The new (possibly renamed) relative path
    new_path: PathBuf,
    /// Whether the file was renamed (name differs from original)
    was_renamed: bool,
}

/// Group files with their rename status by input directory
fn group_files_with_renames(
    input_paths: &[PathBuf],
    original_files: &[PathBuf],
    renamed_files: &[PathBuf],
) -> Vec<(PathBuf, Vec<FileRenameInfo>)> {
    let mut result: Vec<(PathBuf, Vec<FileRenameInfo>)> = Vec::new();

    for input_path in input_paths {
        let mut files_info = Vec::new();

        for (original, renamed) in original_files.iter().zip(renamed_files.iter()) {
            // Check if this file is under this input path
            if let (Ok(orig_relative), Ok(new_relative)) = (
                original.strip_prefix(input_path),
                renamed.strip_prefix(input_path),
            ) {
                let was_renamed = orig_relative.file_name() != new_relative.file_name();
                files_info.push(FileRenameInfo {
                    new_path: new_relative.to_path_buf(),
                    was_renamed,
                });
            }
        }

        if !files_info.is_empty() {
            files_info.sort_by(|a, b| a.new_path.cmp(&b.new_path));
            result.push((input_path.clone(), files_info));
        }
    }

    result
}

/// A tree node for renamed files with rename status
#[derive(Default)]
struct RenameTreeNode {
    children: HashMap<String, RenameTreeNode>,
    is_file: bool,
    was_renamed: bool,
}

/// Build a tree from files with rename info
fn build_rename_tree(files: &[FileRenameInfo]) -> RenameTreeNode {
    let mut root = RenameTreeNode::default();

    for file_info in files {
        let mut current = &mut root;
        let components: Vec<_> = file_info.new_path.components().collect();
        let len = components.len();

        for (idx, component) in components.into_iter().enumerate() {
            let name = component.as_os_str().to_string_lossy().to_string();
            current = current.children.entry(name).or_default();

            // Mark the leaf node (file)
            if idx == len - 1 {
                current.is_file = true;
                current.was_renamed = file_info.was_renamed;
            }
        }
    }

    root
}

/// Show a group of renamed files under an input directory
fn show_rename_group(ui: &mut egui::Ui, input_path: &std::path::Path, files: &[FileRenameInfo]) {
    let display_name = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.display().to_string());

    let parent_path = input_path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let renamed_count = files.iter().filter(|f| f.was_renamed).count();
    let header_text = format!(
        "📁 {} ({} files, {} renamed)",
        display_name,
        files.len(),
        renamed_count
    );

    let header = egui::CollapsingHeader::new(header_text).default_open(true);

    let response = header.show(ui, |ui| {
        let tree = build_rename_tree(files);
        show_rename_tree_children(ui, &tree, 0);
    });

    if !parent_path.is_empty() {
        response.header_response.on_hover_text(&parent_path);
    }
}

/// Show rename tree children
fn show_rename_tree_children(ui: &mut egui::Ui, node: &RenameTreeNode, depth: usize) {
    let mut sorted_children: Vec<_> = node.children.iter().collect();
    sorted_children.sort_by_key(|(k, _)| *k);

    for (child_name, child_node) in sorted_children {
        show_rename_tree_node(ui, child_name, child_node, depth);
    }
}

/// Show a node in the rename tree
fn show_rename_tree_node(ui: &mut egui::Ui, name: &str, node: &RenameTreeNode, depth: usize) {
    if node.children.is_empty() {
        // Leaf node (file) - orange if renamed, green otherwise
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            let color = if node.was_renamed {
                Color32::from_rgb(0xFF, 0xA5, 0x00) // Orange
            } else {
                Color32::LIGHT_GREEN
            };
            ui.colored_label(color, format!("🖼 {name}"));
        });
    } else {
        // Directory with children
        let header_text = format!("📁 {name}");

        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            egui::CollapsingHeader::new(header_text)
                .default_open(depth < 2)
                .show(ui, |ui| {
                    show_rename_tree_children(ui, node, depth + 1);
                });
        });
    }
}

struct CmApp {
    snarl: Snarl<CmNode>,
    style: SnarlStyle,
    /// Cached input paths
    input_paths: Vec<PathBuf>,
    /// Cached image files
    image_files: Vec<PathBuf>,
    /// Path to remove (deferred)
    path_to_remove: Option<PathBuf>,
    /// Whether to clear all (deferred)
    clear_all: bool,
    /// Last error
    last_error: Option<String>,
    /// Whether we've initialized the graph
    initialized: bool,
    /// Whether the logs window is open
    logs_open: bool,
    /// Whether the about window is open
    about_open: bool,
    /// Cached rename preview and key to avoid recomputing every frame
    rename_preview_key: u64,
    rename_preview: Vec<PathBuf>,
}

impl CmApp {
    fn new(_cc: &eframe::CreationContext) -> Self {
        let mut snarl = Snarl::new();

        // Create default nodes
        let inputs_id = snarl.insert_node(egui::pos2(50.0, 100.0), CmNode::Inputs);
        let images_id = snarl.insert_node(egui::pos2(400.0, 100.0), CmNode::ImagePaths);
        // Add RenameFiles node by default and connect images -> rename
        let rename_id = snarl.insert_node(
            egui::pos2(700.0, 100.0),
            CmNode::RenameFiles {
                preview_key: 0,
                preview: Vec::new(),
            },
        );

        // Connect inputs -> images and images -> rename
        snarl.connect(
            egui_snarl::OutPinId {
                node: inputs_id,
                output: 0,
            },
            egui_snarl::InPinId {
                node: images_id,
                input: 0,
            },
        );
        snarl.connect(
            egui_snarl::OutPinId {
                node: images_id,
                output: 0,
            },
            egui_snarl::InPinId {
                node: rename_id,
                input: 0,
            },
        );

        let style = SnarlStyle::new();

        CmApp {
            snarl,
            style,
            input_paths: Vec::new(),
            image_files: Vec::new(),
            path_to_remove: None,
            clear_all: false,
            last_error: None,
            initialized: false,
            logs_open: true,
            about_open: false,
            rename_preview_key: 0,
            rename_preview: Vec::new(),
        }
    }

    fn reload_data(&mut self) {
        // Load input paths
        match inputs::load_inputs(&APP_HOME) {
            Ok(paths) => {
                self.input_paths = paths;
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to load inputs: {}", e));
                self.input_paths.clear();
            }
        }

        // Derive image files from inputs
        match inputs::list_files(&APP_HOME) {
            Ok(files) => {
                self.image_files = files
                    .into_iter()
                    .filter(|p| is_image_file(p.as_path()))
                    .collect();
                self.image_files.sort();
            }
            Err(e) => {
                if self.last_error.is_none() {
                    self.last_error = Some(format!("Failed to list files: {}", e));
                }
                self.image_files.clear();
            }
        }
    }

    fn handle_deferred_actions(&mut self) {
        // Handle clear all
        if self.clear_all {
            self.clear_all = false;
            match inputs::clear_all(&APP_HOME) {
                Ok(()) => {
                    info!("Cleared all inputs");
                    self.reload_data();
                }
                Err(e) => {
                    self.last_error = Some(format!("Failed to clear: {}", e));
                }
            }
        }

        // Handle single path removal
        if let Some(path) = self.path_to_remove.take() {
            match inputs::remove_path(&APP_HOME, &path) {
                Ok(removed) => {
                    if removed {
                        info!("Removed input: {}", path.display());
                    }
                    self.reload_data();
                }
                Err(e) => {
                    self.last_error = Some(format!("Failed to remove: {}", e));
                }
            }
        }
    }
}

impl eframe::App for CmApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Initialize on first frame
        if !self.initialized {
            self.reload_data();
            self.initialized = true;
        }

        // Handle deferred actions from previous frame
        self.handle_deferred_actions();

        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // Refresh button (top-level)
                if ui.button("Refresh").clicked() {
                    self.reload_data();
                }

                // Logs toggle button
                if ui
                    .button(if self.logs_open { "Logs (on)" } else { "Logs" })
                    .clicked()
                {
                    self.logs_open = !self.logs_open;
                }

                // About button
                if ui.button("About").clicked() {
                    self.about_open = !self.about_open;
                }

                // Theme switch
                egui::widgets::global_theme_preference_switch(ui);

                // Right-align the Quit button
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        // Main node graph panel
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut viewer = CmViewer {
                input_paths: &mut self.input_paths,
                image_files: &mut self.image_files,
                path_to_remove: &mut self.path_to_remove,
                clear_all: &mut self.clear_all,
                last_error: &mut self.last_error,
            };

            SnarlWidget::new()
                .id(Id::new("cm-snarl"))
                .style(self.style)
                .show(&mut self.snarl, &mut viewer, ui);
        });

        // Logs window (toggleable)
        if self.logs_open {
            egui::Window::new("Logs")
                .resizable(true)
                .default_pos(egui::pos2(600.0, 400.0))
                .open(&mut self.logs_open)
                .show(ctx, |ui| {
                    let collector = crate::tracing::event_collector();
                    ui.add(egui_tracing::Logs::new(collector));
                });
        }

        // About window
        if self.about_open {
            egui::Window::new("About")
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut self.about_open)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("CM - Creative Memories Photo Manager");
                        ui.add_space(10.0);
                        ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                        ui.label(format!(
                            "Revision: {}",
                            option_env!("GIT_REVISION").unwrap_or("unknown")
                        ));
                        ui.add_space(10.0);
                        ui.label("An app to help manage photos of inventory items.");
                        ui.add_space(10.0);
                        ui.hyperlink_to("GitHub", "https://github.com/TeamDman/cm");
                    });
                });
        }

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

            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
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
                        self.reload_data();
                    }
                    Err(e) => self.last_error = Some(format!("{}", e)),
                }
            }
        }
    }
}
