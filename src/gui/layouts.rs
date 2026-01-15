use crate::app_home::APP_HOME;
use crate::gui::behavior::CmPane;
use eframe::egui::Id;
use egui_tiles::Tree;
use facet::Facet;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Node {
    /// "Pane" or "Container"
    pub node_type: String,
    /// for Pane
    pub pane: Option<String>,
    /// for Container: "Tabs"|"Horizontal"|"Vertical"|"Grid"
    pub kind: Option<String>,
    /// children for Container
    pub children: Option<Vec<Node>>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Layout {
    pub name: String,
    pub root: Node,
}

impl Layout {
    pub fn from_tree(tree: &Tree<CmPane>) -> Option<Layout> {
        let root_id = tree.root()?;
        let root = node_from_tile(tree, root_id);
        let name = "Unnamed".to_string();
        Some(Layout { name, root })
    }

    pub fn apply_to_tree(&self, tree_id: impl Into<Id>) -> Tree<CmPane> {
        let mut tiles = egui_tiles::Tiles::default();

        fn build(node: &Node, tiles: &mut egui_tiles::Tiles<CmPane>) -> egui_tiles::TileId {
            if node.node_type == "Pane" {
                let pane_str = node.pane.as_deref().unwrap_or("InputPaths");
                let pane_obj = CmPane::from_key(pane_str).unwrap_or(CmPane::InputPaths);
                tiles.insert_pane(pane_obj)
            } else {
                let children = node.children.as_deref().unwrap_or(&[]);
                let child_ids: Vec<egui_tiles::TileId> =
                    children.iter().map(|c| build(c, tiles)).collect();
                match node.kind.as_deref().unwrap_or("Tabs") {
                    "Tabs" => tiles.insert_tab_tile(child_ids),
                    "Horizontal" => tiles.insert_horizontal_tile(child_ids),
                    "Vertical" => tiles.insert_vertical_tile(child_ids),
                    "Grid" => tiles.insert_grid_tile(child_ids),
                    _ => tiles.insert_tab_tile(child_ids),
                }
            }
        }

        let root = build(&self.root, &mut tiles);
        Tree::new(tree_id, root, tiles)
    }
}

fn node_from_tile(tree: &Tree<CmPane>, tile_id: egui_tiles::TileId) -> Node {
    if let Some(tile) = tree.tiles.get(tile_id) {
        match tile {
            egui_tiles::Tile::Pane(pane) => Node {
                node_type: "Pane".to_string(),
                pane: Some(pane.to_key().to_string()),
                kind: None,
                children: None,
            },
            egui_tiles::Tile::Container(container) => {
                let kind = match container.kind() {
                    egui_tiles::ContainerKind::Tabs => "Tabs",
                    egui_tiles::ContainerKind::Horizontal => "Horizontal",
                    egui_tiles::ContainerKind::Vertical => "Vertical",
                    egui_tiles::ContainerKind::Grid => "Grid",
                }
                .to_string();
                let children = container
                    .children()
                    .map(|c| node_from_tile(tree, *c))
                    .collect();
                Node {
                    node_type: "Container".to_string(),
                    pane: None,
                    kind: Some(kind),
                    children: Some(children),
                }
            }
        }
    } else {
        // Fallback to an empty tab
        Node {
            node_type: "Container".to_string(),
            pane: None,
            kind: Some("Tabs".to_string()),
            children: Some(vec![]),
        }
    }
}

pub struct LayoutManager {
    pub custom_dir: PathBuf,
    pub preset_dir: PathBuf,
    pub active: Option<String>,
    last_saved_text: Option<String>,
}

impl LayoutManager {
    pub fn new() -> Self {
        let dir = APP_HOME.file_path("layouts");
        let custom_dir = dir.join("custom");
        let preset_dir = dir.join("presets");
        let _ = fs::create_dir_all(&custom_dir);
        let _ = fs::create_dir_all(&preset_dir);
        LayoutManager {
            custom_dir,
            preset_dir,
            active: None,
            last_saved_text: None,
        }
    }

    fn layout_file_for_custom(&self, name: &str) -> PathBuf {
        self.custom_dir
            .join(format!("{}.layout", sanitize_name(name)))
    }

    fn layout_file_for_preset(&self, name: &str) -> PathBuf {
        self.preset_dir
            .join(format!("{}.layout", sanitize_name(name)))
    }

    pub fn list_custom(&self) -> Vec<String> {
        list_names_in_dir(&self.custom_dir)
    }

    pub fn list_presets(&self) -> Vec<String> {
        list_names_in_dir(&self.preset_dir)
    }

    pub fn create_custom_from_layout(
        &mut self,
        name: &str,
        layout: &Layout,
    ) -> eyre::Result<String> {
        let mut new_name = name.to_string();
        // Ensure unique
        let mut i = 1;
        while self.layout_file_for_custom(&new_name).exists() {
            i += 1;
            new_name = format!("{name} {i}");
        }
        let path = self.layout_file_for_custom(&new_name);
        let text = facet_json::to_string(layout)?;
        fs::write(&path, text)?;
        Ok(new_name)
    }

    pub fn save_preset(&self, name: &str, layout: &Layout) -> eyre::Result<()> {
        let path = self.layout_file_for_preset(name);
        let text = facet_json::to_string(layout)?;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        f.write_all(text.as_bytes())?;
        Ok(())
    }

    pub fn save_active(&mut self, layout: &Layout) -> eyre::Result<()> {
        if let Some(active) = &self.active {
            let path = self.layout_file_for_custom(active);
            let text = facet_json::to_string(layout)?;
            let mut f = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;
            f.write_all(text.as_bytes())?;
            self.last_saved_text = Some(text);
        }
        Ok(())
    }

    pub fn load_named(&self, name: &str) -> eyre::Result<Layout> {
        let path_custom = self.layout_file_for_custom(name);
        if path_custom.exists() {
            let s = fs::read_to_string(&path_custom)?;
            let l: Layout = facet_json::from_str(&s)?;
            return Ok(l);
        }
        let path_preset = self.layout_file_for_preset(name);
        if path_preset.exists() {
            let s = fs::read_to_string(&path_preset)?;
            let l: Layout = facet_json::from_str(&s)?;
            return Ok(l);
        }
        Err(eyre::eyre!("Unknown layout: {}", name))
    }

    /// Activate a preset by copying it into a new custom layout, then returning its new name.
    pub fn activate_preset_as_custom(
        &mut self,
        preset_name: &str,
        _tree_id: impl Into<Id>,
    ) -> eyre::Result<String> {
        let layout = self.load_named(preset_name)?;
        let new_name = format!("Custom from {preset_name}");
        let new_name = self.create_custom_from_layout(&new_name, &layout)?;
        self.active = Some(new_name.clone());
        Ok(new_name)
    }

    pub fn set_active(&mut self, name: &str) {
        self.active = Some(name.to_string());
        // reset last_saved so first save will write to disk
        self.last_saved_text = None;
    }

    pub fn active_name(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn delete_active(&mut self) -> eyre::Result<()> {
        if let Some(active) = &self.active {
            let path = self.layout_file_for_custom(active);
            if path.exists() {
                fs::remove_file(path)?;
                self.active = None;
            }
        }
        Ok(())
    }

    /// Compare layout text and save if changed
    pub fn maybe_autosave(&mut self, layout: &Layout) -> eyre::Result<()> {
        let text = facet_json::to_string(layout)?;
        if self.last_saved_text.as_deref() != Some(&text) {
            self.save_active(layout)?;
        }
        Ok(())
    }
}

fn list_names_in_dir(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(iter) = fs::read_dir(dir) {
        for e in iter.flatten() {
            let p = e.path();
            if let Some(ext) = p.extension()
                && ext == "layout"
                && let Some(stem) = p.file_stem().and_then(|s| s.to_str())
            {
                out.push(desanitize_name(stem));
            }
        }
    }
    out
}

fn sanitize_name(name: &str) -> String {
    name.replace(' ', "_")
}

fn desanitize_name(name: &str) -> String {
    name.replace('_', " ")
}
