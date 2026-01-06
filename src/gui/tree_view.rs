//! Tree view helper functions for displaying file hierarchies

use eframe::egui::{self, Color32};
use std::collections::HashMap;
use std::path::PathBuf;

/// A simple tree node for displaying paths hierarchically
#[derive(Default)]
pub struct TreeNode {
    pub children: HashMap<String, TreeNode>,
    pub is_file: bool,
}

/// Build a tree from relative paths
pub fn build_path_tree(paths: &[PathBuf]) -> TreeNode {
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

/// Show tree children (skipping the root level)
pub fn show_tree_children(ui: &mut egui::Ui, node: &TreeNode, depth: usize) {
    let mut sorted_children: Vec<_> = node.children.iter().collect();
    sorted_children.sort_by_key(|(k, _)| *k);

    for (child_name, child_node) in sorted_children {
        show_tree_node(ui, child_name, child_node, depth, None);
    }
}

/// Show a single tree node
pub fn show_tree_node(
    ui: &mut egui::Ui,
    name: &str,
    node: &TreeNode,
    depth: usize,
    file_color: Option<Color32>,
) {
    if node.children.is_empty() {
        // Leaf node (file)
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            let color = file_color.unwrap_or(Color32::LIGHT_GREEN);
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
                    show_tree_children(ui, node, depth + 1);
                });
        });
    }
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
pub fn show_input_group(ui: &mut egui::Ui, input_path: &std::path::Path, relative_files: &[PathBuf]) {
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
        let tree = build_path_tree(relative_files);
        show_tree_children(ui, &tree, 0);
    });

    if !parent_path.is_empty() {
        response.header_response.on_hover_text(&parent_path);
    }
}

/// Info about a file and whether it was renamed / is too long
#[derive(Clone)]
pub struct FileRenameInfo {
    /// The original relative path
    pub original_path: PathBuf,
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
            if let (Ok(orig_relative), Ok(new_relative)) = (
                original.strip_prefix(input_path),
                renamed.strip_prefix(input_path),
            ) {
                let orig_name = orig_relative.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let new_name = new_relative.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let was_renamed = orig_name != new_name;
                let is_too_long = new_name.len() > max_name_length;

                files_info.push(FileRenameInfo {
                    original_path: orig_relative.to_path_buf(),
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
                current.full_path = Some(input_path.join(&file_info.new_path));
            }
        }
    }

    root
}

/// Show rename tree children
pub fn show_rename_tree_children(ui: &mut egui::Ui, node: &RenameTreeNode, depth: usize) {
    let mut sorted_children: Vec<_> = node.children.iter().collect();
    sorted_children.sort_by_key(|(k, _)| *k);

    for (child_name, child_node) in sorted_children {
        show_rename_tree_node(ui, child_name, child_node, depth);
    }
}

/// Show a node in the rename tree
pub fn show_rename_tree_node(ui: &mut egui::Ui, name: &str, node: &RenameTreeNode, depth: usize) {
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

/// Show a group of renamed files under an input directory
pub fn show_rename_group(
    ui: &mut egui::Ui,
    input_path: &std::path::Path,
    files: &[FileRenameInfo],
    max_name_length: usize,
) {
    let display_name = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.display().to_string());

    let parent_path = input_path
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
        let tree = build_rename_tree(files, input_path);
        show_rename_tree_children(ui, &tree, 0);
    });

    if !parent_path.is_empty() {
        response.header_response.on_hover_text(&parent_path);
    }
}
