//! Tree view helper functions for displaying file hierarchies

use crate::gui::state::CachedImageInfo;
use eframe::egui::Color32;
use eframe::egui::Sense;
use eframe::egui::TextureHandle;
use eframe::egui::TextureOptions;
use eframe::egui::{self};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write;

#[expect(clippy::cast_precision_loss)]
fn depth_to_space(depth: usize) -> f32 {
    depth as f32 * 16.0
}
use std::path::Path;
use std::path::PathBuf;
#[cfg(windows)]
use teamy_windows::shell::select::open_folder_and_select_items;
use tracing::debug;

/// A simple tree node for displaying paths hierarchically
#[derive(Default, Debug)]
pub struct TreeNode {
    pub children: HashMap<String, TreeNode>,
    pub is_file: bool,
    /// Full path to the file (only set for leaf nodes)
    pub full_path: Option<PathBuf>,
}

/// Context for rendering tree nodes with image cache
#[expect(missing_debug_implementations)]
pub struct TreeRenderContext<'a> {
    pub image_cache: &'a HashMap<PathBuf, CachedImageInfo>,
    pub images_loading: &'a HashSet<PathBuf>,
    pub thumbnail_textures: &'a mut HashMap<PathBuf, TextureHandle>,
}

/// Build a tree from relative paths, storing full paths for files
#[must_use]
pub fn build_path_tree(paths: &[PathBuf], base_path: &Path) -> TreeNode {
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
#[derive(Default, Debug)]
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
    show_tree_children_with_cache(ui, node, depth, selected_path, None)
}

/// Show tree children with optional image cache context
pub fn show_tree_children_with_cache(
    ui: &mut egui::Ui,
    node: &TreeNode,
    depth: usize,
    selected_path: Option<&PathBuf>,
    ctx: Option<&mut TreeRenderContext<'_>>,
) -> TreeResult {
    let mut result = TreeResult::default();
    let mut sorted_children: Vec<_> = node.children.iter().collect();
    sorted_children.sort_by_key(|(k, _)| *k);

    // We need to handle ctx mutability carefully
    if let Some(ctx) = ctx {
        for (child_name, child_node) in sorted_children {
            let child_result = show_tree_node_with_cache(
                ui,
                child_name,
                child_node,
                depth,
                None,
                selected_path,
                Some(ctx),
            );
            if child_result.clicked_path.is_some() {
                result = child_result;
            }
        }
    } else {
        for (child_name, child_node) in sorted_children {
            let child_result = show_tree_node_with_cache(
                ui,
                child_name,
                child_node,
                depth,
                None,
                selected_path,
                None,
            );
            if child_result.clicked_path.is_some() {
                result = child_result;
            }
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
    show_tree_node_with_cache(ui, name, node, depth, file_color, selected_path, None)
}

/// Show a single tree node with optional image cache, returning any clicked file path
#[expect(clippy::too_many_lines)]
pub fn show_tree_node_with_cache(
    ui: &mut egui::Ui,
    name: &str,
    node: &TreeNode,
    depth: usize,
    file_color: Option<Color32>,
    selected_path: Option<&PathBuf>,
    ctx: Option<&mut TreeRenderContext<'_>>,
) -> TreeResult {
    let mut result = TreeResult::default();

    if node.children.is_empty() {
        // Leaf node (file) - make it clickable
        ui.horizontal(|ui| {
            ui.add_space(depth_to_space(depth));
            let color = file_color.unwrap_or(Color32::LIGHT_GREEN);

            // Check if this node is selected
            let is_selected = node
                .full_path
                .as_ref()
                .is_some_and(|p| Some(p) == selected_path);

            // Build the label text with image info if available
            let (label_text, is_loading, cached_info) = if let Some(ref path) = node.full_path {
                if let Some(ref ctx) = ctx {
                    if let Some(info) = ctx.image_cache.get(path) {
                        // Show dimensions and size
                        let size_str = format_size(info.file_size);
                        let label =
                            format!("🖼 {} ({} {}x{})", name, size_str, info.width, info.height);
                        (label, false, Some(info.clone()))
                    } else if ctx.images_loading.contains(path) {
                        (format!("⏳ {name}"), true, None)
                    } else {
                        (format!("🖼 {name}"), false, None)
                    }
                } else {
                    (format!("🖼 {name}"), false, None)
                }
            } else {
                (format!("🖼 {name}"), false, None)
            };

            let response = if is_selected {
                // Highlighted when selected
                ui.add(
                    egui::Label::new(egui::RichText::new(&label_text).color(color).underline())
                        .sense(Sense::click()),
                )
            } else if is_loading {
                ui.add(
                    egui::Label::new(egui::RichText::new(&label_text).color(Color32::GRAY))
                        .sense(Sense::click()),
                )
            } else {
                ui.add(
                    egui::Label::new(egui::RichText::new(&label_text).color(color))
                        .sense(Sense::click()),
                )
            };

            if response.clicked() {
                result.clicked_path.clone_from(&node.full_path);
            }

            // Tooltip with thumbnail and path
            if let Some(ref path) = node.full_path {
                let hover_response = if let Some(info) = cached_info {
                    if let Some(ctx) = ctx {
                        // Show image tooltip with thumbnail
                        let texture =
                            ctx.thumbnail_textures
                                .entry(path.clone())
                                .or_insert_with(|| {
                                    // Load thumbnail texture
                                    if let Ok(image) = image::load_from_memory(&info.thumbnail_data)
                                    {
                                        let size = [image.width() as _, image.height() as _];
                                        let rgba = image.to_rgba8();
                                        let pixels = rgba.as_flat_samples();
                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                            size,
                                            pixels.as_slice(),
                                        );
                                        ui.ctx().load_texture(
                                            format!("thumb_{}", path.display()),
                                            color_image,
                                            TextureOptions::default(),
                                        )
                                    } else {
                                        // Fallback: 1x1 transparent texture
                                        ui.ctx().load_texture(
                                            "thumb_fallback",
                                            egui::ColorImage::new(
                                                [1, 1],
                                                vec![Color32::TRANSPARENT],
                                            ),
                                            TextureOptions::default(),
                                        )
                                    }
                                });

                        response.on_hover_ui(|ui| {
                            ui.vertical(|ui| {
                                ui.image((texture.id(), texture.size_vec2()));
                                ui.add_space(4.0);
                                ui.label(format!("{}x{}", info.width, info.height));
                                ui.label(format_size(info.file_size));
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(path.display().to_string()).small());
                            });
                        })
                    } else {
                        response.on_hover_text(path.display().to_string())
                    }
                } else {
                    response.on_hover_text(path.display().to_string())
                };

                // Context menu to open file in Explorer/Finder (always available)
                hover_response.context_menu(|ui| {
                    if ui.button("Open in explorer").clicked() {
                        open_in_explorer(path);
                        ui.close();
                    }
                });
            }
        });
    } else {
        // Directory with children
        let header_text = format!("📁 {name}");

        ui.horizontal(|ui| {
            ui.add_space(depth_to_space(depth));
            egui::CollapsingHeader::new(header_text)
                .default_open(depth < 2)
                .show(ui, |ui| {
                    result = show_tree_children_with_cache(ui, node, depth + 1, selected_path, ctx);
                });
        });
    }

    result
}

/// Format file size in human-readable form
#[expect(clippy::cast_precision_loss)]
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0}KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}

/// Reveal `path` in the host file manager (Explorer/Finder/xdg-open).
fn open_in_explorer(path: &Path) {
    debug!("Opening in explorer: {}", path.display());

    #[cfg(windows)]
    {
        // Pass the path as a single-element iterator. Passing `path` directly can
        // cause the function to iterate the path's components (treating it like an
        // iterator), which results in incorrect behavior (e.g., paths like `C:\`).
        if let Err(e) = open_folder_and_select_items(&[path]) {
            tracing::error!("Failed to open in explorer: {:?}", e);
        }
    }
    #[cfg(not(windows))]
    tracing::warn!("Not implemented for this platform - open in explorer");
}

/// Group image files by which input directory they belong to.
/// Returns a list of (`input_path`, `relative_file_paths`) tuples.
#[must_use]
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
    input_path: &Path,
    relative_files: &[PathBuf],
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    show_input_group_with_cache(ui, input_path, relative_files, selected_path, None)
}

/// Show a group of files under an input directory with optional image cache
pub fn show_input_group_with_cache(
    ui: &mut egui::Ui,
    input_path: &Path,
    relative_files: &[PathBuf],
    selected_path: Option<&PathBuf>,
    ctx: Option<&mut TreeRenderContext<'_>>,
) -> TreeResult {
    let mut result = TreeResult::default();

    let display_name = input_path.file_name().map_or_else(
        || input_path.display().to_string(),
        |s| s.to_string_lossy().to_string(),
    );

    let parent_path = input_path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let header_text = format!("📁 {} ({} images)", display_name, relative_files.len());

    let header = egui::CollapsingHeader::new(header_text).default_open(true);

    let response = header.show(ui, |ui| {
        let tree = build_path_tree(relative_files, input_path);
        result = show_tree_children_with_cache(ui, &tree, 0, selected_path, ctx);
    });

    if !parent_path.is_empty() {
        let header_resp = response.header_response.on_hover_text(&parent_path);
        header_resp.context_menu(|ui| {
            if ui.button("Open in explorer").clicked() {
                open_in_explorer(input_path);
                ui.close();
            }
        });
    }

    result
}

/// Info about a file and whether it was renamed / is too long
#[derive(Clone, Debug)]
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
#[must_use]
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
                let new_name = new_relative
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
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
#[derive(Default, Debug)]
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
#[must_use]
pub fn build_rename_tree(files: &[FileRenameInfo], input_path: &Path) -> RenameTreeNode {
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
            ui.add_space(depth_to_space(depth));
            let color = if node.is_too_long {
                Color32::RED
            } else if node.was_renamed {
                Color32::from_rgb(0xFF, 0xA5, 0x00) // Orange
            } else {
                Color32::LIGHT_GREEN
            };

            // Check if this node is selected (compare against original input path)
            let is_selected = node
                .original_input_path
                .as_ref()
                .is_some_and(|p| Some(p) == selected_path);

            let label_text = format!("🖼 {} ({})", name, name.len());
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
                result.clicked_path.clone_from(&node.original_input_path);
            }

            // Tooltip with output path info
            if let Some(ref path) = node.full_path {
                let mut tooltip = format!("Output: {}", path.display());
                if let Some(ref orig) = node.original_input_path {
                    let _ = write!(tooltip, "\nInput: {}", orig.display());
                }
                let response = response.on_hover_text(tooltip);

                // Context menu to open the file in Explorer/Finder (prefer output path)
                if let Some(open_path) = node
                    .full_path
                    .as_ref()
                    .or(node.original_input_path.as_ref())
                {
                    response.context_menu(|ui| {
                        if ui.button("Open in explorer").clicked() {
                            if open_path.exists() {
                                open_in_explorer(open_path);
                            } else {
                                tracing::error!(
                                    "Cannot open in explorer: path does not exist: {}",
                                    open_path.display()
                                );
                            }
                            ui.close();
                        }
                    });
                }
            } else {
                // If only original_input_path is available (no full_path tooltip), allow context menu on the label
                if let Some(open_path) = node.original_input_path.as_ref() {
                    response.context_menu(|ui| {
                        if ui.button("Open in explorer").clicked() {
                            if open_path.exists() {
                                open_in_explorer(open_path);
                            } else {
                                tracing::error!(
                                    "Cannot open in explorer: path does not exist: {}",
                                    open_path.display()
                                );
                            }
                            ui.close();
                        }
                    });
                }
            }
        });
    } else {
        // Directory with children
        let header_text = format!("📁 {name}");

        ui.horizontal(|ui| {
            ui.add_space(depth_to_space(depth));
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
pub fn show_rename_group(
    ui: &mut egui::Ui,
    input_path: &Path,
    files: &[FileRenameInfo],
    max_name_length: usize,
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    show_rename_group_with_output_path(
        ui,
        input_path,
        input_path,
        files,
        max_name_length,
        selected_path,
    )
}

/// Show a group of renamed files with a custom output path display
pub fn show_rename_group_with_output_path(
    ui: &mut egui::Ui,
    _input_path: &Path,
    output_path: &Path,
    files: &[FileRenameInfo],
    max_name_length: usize,
    selected_path: Option<&PathBuf>,
) -> TreeResult {
    let mut result = TreeResult::default();

    let display_name = output_path.file_name().map_or_else(
        || output_path.display().to_string(),
        |s| s.to_string_lossy().to_string(),
    );

    let parent_path = output_path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let renamed_count = files.iter().filter(|f| f.was_renamed).count();
    let too_long_count = files.iter().filter(|f| f.is_too_long).count();

    let mut header_text = format!("📁 {} ({} files", display_name, files.len(),);
    if renamed_count > 0 {
        let _ = write!(header_text, ", {renamed_count} renamed");
    }
    if too_long_count > 0 {
        let _ = write!(
            header_text,
            ", {too_long_count} too long (>{max_name_length} chars)"
        );
    }
    header_text.push(')');

    let header = egui::CollapsingHeader::new(header_text).default_open(true);

    let response = header.show(ui, |ui| {
        let tree = build_rename_tree(files, output_path);
        result = show_rename_tree_children(ui, &tree, 0, selected_path);
    });

    if !parent_path.is_empty() {
        let header_resp = response.header_response.on_hover_text(&parent_path);
        header_resp.context_menu(|ui| {
            if ui.button("Open in explorer").clicked() {
                open_in_explorer(output_path);
                ui.close();
            }
        });
    }

    result
}
