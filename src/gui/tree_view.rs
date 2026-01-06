//! Tree view helper functions for displaying file hierarchies

use eframe::egui::{self, Color32, Sense};
use std::collections::HashMap;
use std::path::PathBuf;

/// A simple tree node for displaying paths hierarchically
#[derive(Default)]
pub struct TreeNode {
    pub children: HashMap<String, TreeNode>,
    pub is_file: bool,
    /// Full path to the file (only set for leaf nodes)
    pub full_path: Option<PathBuf>,
}

/// Build a tree from relative paths, storing full paths for files
pub fn build_path_tree(paths: &[PathBuf], base_path: &std::path::Path) -> TreeNode {
    let mut root = TreeNode::default();

    for path in paths {
        let mut current = &mut root;
        let components: Vec<_> = path.components().collect();
        let len = components.len();

        for (idx, component) in components.into_iter().enumerate() {
            let name = component.as_os_str().to_string_lossy().to_string();
            current = current.children.entry(name).or_default();

            // Mark leaf node with full path
            if idx == len - 1 {
                current.is_file = true;
                current.full_path = Some(base_path.join(path));
            }
        }
    }

    root
}

/// Result of showing a tree - contains the clicked file path if any
#[derive(Default)]
pub struct TreeResult {
    pub clicked_path: Option<PathBuf>,
}

/// Show tree children (skipping the root level), returning any clicked file
pub fn show_tree_children(
    ui: &mut egui::Ui,
    node: &TreeNode,
    depth: usize,
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    let mut result = TreeResult::default();
    let mut sorted_children: Vec<_> = node.children.iter().collect();
    sorted_children.sort_by_key(|(k, _)| *k);

    for (child_name, child_node) in sorted_children {
        let child_result = show_tree_node(ui, child_name, child_node, depth, None, selected_path);
        if child_result.clicked_path.is_some() {
            result = child_result;
        }
    }

    result
}

/// Show a single tree node, returning any clicked file path
pub fn show_tree_node(
    ui: &mut egui::Ui,
    name: &str,
    node: &TreeNode,
    depth: usize,
    file_color: Option<Color32>,
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    let mut result = TreeResult::default();

    if node.children.is_empty() {
        // Leaf node (file) - make it clickable
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            let color = file_color.unwrap_or(Color32::LIGHT_GREEN);
            
            // Check if this node is selected
            let is_selected = node.full_path.as_ref().is_some_and(|p| Some(p) == selected_path);
            
            let label_text = format!("🖼 {name}");
            let response = if is_selected {
                // Highlighted when selected
                ui.add(
                    egui::Label::new(egui::RichText::new(&label_text).color(color).underline())
                        .sense(Sense::click()),
                )
            } else {
                ui.add(
                    egui::Label::new(egui::RichText::new(&label_text).color(color))
                        .sense(Sense::click()),
                )
            };

            if response.clicked() {
                result.clicked_path = node.full_path.clone();
            }

            // Tooltip with full path
            if let Some(ref path) = node.full_path {
                response.on_hover_text(path.display().to_string());
            }
        });
    } else {
        // Directory with children
        let header_text = format!("📁 {name}");

        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            egui::CollapsingHeader::new(header_text)
                .default_open(depth < 2)
                .show(ui, |ui| {
                    result = show_tree_children(ui, node, depth + 1, selected_path);
                });
        });
    }

    result
}

/// Group image files by which input directory they belong to.
/// Returns a list of (input_path, relative_file_paths) tuples.
pub fn group_files_by_input(
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

/// Show a group of files under an input directory
pub fn show_input_group(
    ui: &mut egui::Ui,
    input_path: &std::path::Path,
    relative_files: &[PathBuf],
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    let mut result = TreeResult::default();

    let display_name = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.display().to_string());

    let parent_path = input_path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let header_text = format!("📁 {} ({} images)", display_name, relative_files.len());

    let header = egui::CollapsingHeader::new(header_text).default_open(true);

    let response = header.show(ui, |ui| {
        let tree = build_path_tree(relative_files, input_path);
        result = show_tree_children(ui, &tree, 0, selected_path);
    });

    if !parent_path.is_empty() {
        response.header_response.on_hover_text(&parent_path);
    }

    result
}

/// Info about a file and whether it was renamed / is too long
#[derive(Clone)]
pub struct FileRenameInfo {
    /// The original input file path (absolute)
    pub original_input_path: PathBuf,
    /// The new (possibly renamed) relative path
    pub new_path: PathBuf,
    /// Whether the file was renamed (name differs from original)
    pub was_renamed: bool,
    /// Whether the file name is too long
    pub is_too_long: bool,
}

/// Group files with their rename status by input directory
pub fn group_files_with_renames(
    input_paths: &[PathBuf],
    original_files: &[PathBuf],
    renamed_files: &[PathBuf],
    max_name_length: usize,
) -> Vec<(PathBuf, Vec<FileRenameInfo>)> {
    let mut result: Vec<(PathBuf, Vec<FileRenameInfo>)> = Vec::new();

    for input_path in input_paths {
        let mut files_info = Vec::new();

        for (original, renamed) in original_files.iter().zip(renamed_files.iter()) {
            if let (Ok(_orig_relative), Ok(new_relative)) = (
                original.strip_prefix(input_path),
                renamed.strip_prefix(input_path),
            ) {
                let orig_name = original.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let new_name = new_relative.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let was_renamed = orig_name != new_name;
                let is_too_long = new_name.len() > max_name_length;

                files_info.push(FileRenameInfo {
                    original_input_path: original.clone(),
                    new_path: new_relative.to_path_buf(),
                    was_renamed,
                    is_too_long,
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
pub struct RenameTreeNode {
    pub children: HashMap<String, RenameTreeNode>,
    pub is_file: bool,
    pub was_renamed: bool,
    pub is_too_long: bool,
    /// The original input file path (for selection tracking)
    pub original_input_path: Option<PathBuf>,
    pub full_path: Option<PathBuf>,
}

/// Build a tree from files with rename info
pub fn build_rename_tree(files: &[FileRenameInfo], input_path: &std::path::Path) -> RenameTreeNode {
    let mut root = RenameTreeNode::default();

    for file_info in files {
        let mut current = &mut root;
        let components: Vec<_> = file_info.new_path.components().collect();
        let len = components.len();

        for (idx, component) in components.into_iter().enumerate() {
            let name = component.as_os_str().to_string_lossy().to_string();
            current = current.children.entry(name).or_default();

            if idx == len - 1 {
                current.is_file = true;
                current.was_renamed = file_info.was_renamed;
                current.is_too_long = file_info.is_too_long;
                current.original_input_path = Some(file_info.original_input_path.clone());
                current.full_path = Some(input_path.join(&file_info.new_path));
            }
        }
    }

    root
}

/// Show rename tree children
pub fn show_rename_tree_children(
    ui: &mut egui::Ui,
    node: &RenameTreeNode,
    depth: usize,
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    let mut result = TreeResult::default();
    let mut sorted_children: Vec<_> = node.children.iter().collect();
    sorted_children.sort_by_key(|(k, _)| *k);

    for (child_name, child_node) in sorted_children {
        let child_result = show_rename_tree_node(ui, child_name, child_node, depth, selected_path);
        if child_result.clicked_path.is_some() {
            result = child_result;
        }
    }

    result
}

/// Show a node in the rename tree
pub fn show_rename_tree_node(
    ui: &mut egui::Ui,
    name: &str,
    node: &RenameTreeNode,
    depth: usize,
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    let mut result = TreeResult::default();

    if node.children.is_empty() {
        // Leaf node (file) - red if too long, orange if renamed, green otherwise
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            let color = if node.is_too_long {
                Color32::RED
            } else if node.was_renamed {
                Color32::from_rgb(0xFF, 0xA5, 0x00) // Orange
            } else {
                Color32::LIGHT_GREEN
            };

            // Check if this node is selected (compare against original input path)
            let is_selected = node.original_input_path.as_ref().is_some_and(|p| Some(p) == selected_path);

            let label_text = format!("🖼 {name}");
            let response = if is_selected {
                ui.add(
                    egui::Label::new(egui::RichText::new(&label_text).color(color).underline())
                        .sense(Sense::click()),
                )
            } else {
                ui.add(
                    egui::Label::new(egui::RichText::new(&label_text).color(color))
                        .sense(Sense::click()),
                )
            };

            if response.clicked() {
                // Return the original input path so we can select the same file in both trees
                result.clicked_path = node.original_input_path.clone();
            }

            // Tooltip with output path info
            if let Some(ref path) = node.full_path {
                let mut tooltip = format!("Output: {}", path.display());
                if let Some(ref orig) = node.original_input_path {
                    tooltip.push_str(&format!("\nInput: {}", orig.display()));
                }
                response.on_hover_text(tooltip);
            }
        });
    } else {
        // Directory with children
        let header_text = format!("📁 {name}");

        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            egui::CollapsingHeader::new(header_text)
                .default_open(depth < 2)
                .show(ui, |ui| {
                    result = show_rename_tree_children(ui, node, depth + 1, selected_path);
                });
        });
    }

    result
}

/// Show a group of renamed files under an input directory
#[allow(dead_code)]
pub fn show_rename_group(
    ui: &mut egui::Ui,
    input_path: &std::path::Path,
    files: &[FileRenameInfo],
    max_name_length: usize,
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    show_rename_group_with_output_path(ui, input_path, input_path, files, max_name_length, selected_path)
}

/// Show a group of renamed files with a custom output path display
pub fn show_rename_group_with_output_path(
    ui: &mut egui::Ui,
    _input_path: &std::path::Path,
    output_path: &std::path::Path,
    files: &[FileRenameInfo],
    max_name_length: usize,
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    let mut result = TreeResult::default();

    let display_name = output_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| output_path.display().to_string());

    let parent_path = output_path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let renamed_count = files.iter().filter(|f| f.was_renamed).count();
    let too_long_count = files.iter().filter(|f| f.is_too_long).count();
    
    let mut header_text = format!(
        "📁 {} ({} files",
        display_name,
        files.len(),
    );
    if renamed_count > 0 {
        header_text.push_str(&format!(", {} renamed", renamed_count));
    }
    if too_long_count > 0 {
        header_text.push_str(&format!(", {} too long (>{} chars)", too_long_count, max_name_length));
    }
    header_text.push(')');

    let header = egui::CollapsingHeader::new(header_text).default_open(true);

    let response = header.show(ui, |ui| {
        let tree = build_rename_tree(files, output_path);
        result = show_rename_tree_children(ui, &tree, 0, selected_path);
    });

    if !parent_path.is_empty() {
        response.header_response.on_hover_text(&parent_path);
    }

    result
}
