//! Tile behavior and pane definitions for `egui_tiles`

use crate::gui::state::AppState;
use crate::gui::tiles;
use eframe::egui::TextureHandle;
use eframe::egui::{self};
use std::collections::HashMap;
use std::path::PathBuf;

/// The different types of panes in our application
#[derive(Clone, Debug)]
pub enum CmPane {
    /// Input paths management
    InputPaths,
    /// Tree view of input images
    InputImages,
    /// Image manipulation settings
    ImageManipulation,
    /// Rename rules management
    RenameRules,
    /// Max name length setting
    MaxNameLength,
    /// Output preview with renamed files
    OutputPreview,
    /// Input image preview
    InputImagePreview,
    /// Threshold preview (binarized)
    ThresholdPreview,
    /// Output image preview  
    OutputImagePreview,
    /// Image description/EXIF data
    ImageDescription,
    /// Product Search (Searchspring)
    ProductSearch,
    /// V2 plan builder summary
    Plan,
    /// V2 mom-friendly guided workflow
    StudioGuide,
}

impl CmPane {
    /// Get the title for this pane
    pub fn title(&self) -> &'static str {
        match self {
            CmPane::InputPaths => "Input Paths",
            CmPane::InputImages => "Input Images",
            CmPane::ImageManipulation => "Image Manipulation",
            CmPane::RenameRules => "Rename Rules",
            CmPane::MaxNameLength => "Max Name Length",
            CmPane::OutputPreview => "Output Preview",
            CmPane::InputImagePreview => "Input Preview",
            CmPane::ThresholdPreview => "Threshold Preview",
            CmPane::OutputImagePreview => "Output Preview Image",
            CmPane::ImageDescription => "Image Description",
            CmPane::ProductSearch => "Product Search",
            CmPane::Plan => "Plan",
            CmPane::StudioGuide => "Studio Guide",
        }
    }

    /// Stable key used for serialization/deserialization
    pub fn to_key(&self) -> &'static str {
        match self {
            CmPane::InputPaths => "InputPaths",
            CmPane::InputImages => "InputImages",
            CmPane::ImageManipulation => "ImageManipulation",
            CmPane::RenameRules => "RenameRules",
            CmPane::MaxNameLength => "MaxNameLength",
            CmPane::OutputPreview => "OutputPreview",
            CmPane::InputImagePreview => "InputImagePreview",
            CmPane::ThresholdPreview => "ThresholdPreview",
            CmPane::OutputImagePreview => "OutputImagePreview",
            CmPane::ImageDescription => "ImageDescription",
            CmPane::ProductSearch => "ProductSearch",
            CmPane::Plan => "Plan",
            CmPane::StudioGuide => "StudioGuide",
        }
    }

    /// Construct from a key produced by `to_key`.
    pub fn from_key(s: &str) -> Option<Self> {
        Some(match s {
            "InputPaths" => CmPane::InputPaths,
            "InputImages" => CmPane::InputImages,
            "ImageManipulation" => CmPane::ImageManipulation,
            "RenameRules" => CmPane::RenameRules,
            "MaxNameLength" => CmPane::MaxNameLength,
            "OutputPreview" => CmPane::OutputPreview,
            "InputImagePreview" => CmPane::InputImagePreview,
            "ThresholdPreview" => CmPane::ThresholdPreview,
            "OutputImagePreview" => CmPane::OutputImagePreview,
            "ImageDescription" => CmPane::ImageDescription,
            "ProductSearch" => CmPane::ProductSearch,
            "Plan" => CmPane::Plan,
            "StudioGuide" => CmPane::StudioGuide,
            _ => return None,
        })
    }
}

/// Behavior implementation for our tile tree
pub struct CmBehavior<'a> {
    pub state: &'a mut AppState,
    pub output_texture: &'a mut Option<TextureHandle>,
    pub output_texture_path: &'a mut Option<PathBuf>,
    pub threshold_texture: &'a mut Option<TextureHandle>,
    pub threshold_texture_path: &'a mut Option<PathBuf>,
    pub input_pan_zoom: &'a mut tiles::PanZoomState,
    pub threshold_pan_zoom: &'a mut tiles::PanZoomState,
    pub output_pan_zoom: &'a mut tiles::PanZoomState,
    pub thumbnail_textures: &'a mut HashMap<PathBuf, TextureHandle>,
}

impl egui_tiles::Behavior<CmPane> for CmBehavior<'_> {
    fn tab_title_for_pane(&mut self, pane: &CmPane) -> egui::WidgetText {
        pane.title().into()
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut CmPane,
    ) -> egui_tiles::UiResponse {
        match pane {
            CmPane::InputPaths => tiles::draw_input_paths_tile(ui, self.state),
            CmPane::InputImages => {
                tiles::draw_input_images_tile(ui, self.state, self.thumbnail_textures);
            }
            CmPane::ImageManipulation => tiles::draw_image_manipulation_tile(ui, self.state),
            CmPane::RenameRules => tiles::draw_rename_rules_tile(ui, self.state),
            CmPane::MaxNameLength => tiles::draw_max_name_length_tile(ui, self.state),
            CmPane::OutputPreview => tiles::draw_output_preview_tile(ui, self.state),
            CmPane::InputImagePreview => {
                tiles::draw_input_image_preview_tile(ui, self.state, self.input_pan_zoom);
            }
            CmPane::ThresholdPreview => tiles::draw_threshold_preview_tile(
                ui,
                self.state,
                self.threshold_texture,
                self.threshold_texture_path,
                self.threshold_pan_zoom,
            ),
            CmPane::OutputImagePreview => tiles::draw_output_image_preview_tile(
                ui,
                self.state,
                self.output_texture,
                self.output_texture_path,
                self.output_pan_zoom,
            ),
            CmPane::ImageDescription => tiles::draw_image_description_tile(ui, self.state),
            CmPane::ProductSearch => tiles::draw_product_search_tile(ui, self.state),
            CmPane::Plan => tiles::draw_plan_tile(ui, self.state),
            CmPane::StudioGuide => tiles::draw_studio_guide_tile(ui, self.state),
        }

        // For now, no drag response
        egui_tiles::UiResponse::None
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        24.0
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        2.0
    }
}

/// Create the default tile tree layout
pub fn create_default_tree() -> egui_tiles::Tree<CmPane> {
    create_tree(false)
}

/// Create the v2 tile tree layout with a plan pane.
#[cfg(test)]
pub fn create_v2_tree() -> egui_tiles::Tree<CmPane> {
    create_tree(true)
}

/// Create a standalone product-search layout outside the studios.
pub fn create_product_search_tree() -> egui_tiles::Tree<CmPane> {
    let mut tiles = egui_tiles::Tiles::default();
    let root = tiles.insert_pane(CmPane::ProductSearch);
    egui_tiles::Tree::new("cm_product_search_tree", root, tiles)
}

fn create_tree(include_plan: bool) -> egui_tiles::Tree<CmPane> {
    let mut tiles = egui_tiles::Tiles::default();

    // Create panes
    let input_paths_id = tiles.insert_pane(CmPane::InputPaths);
    let input_images_id = tiles.insert_pane(CmPane::InputImages);
    let image_manipulation_id = tiles.insert_pane(CmPane::ImageManipulation);
    let rename_rules_id = tiles.insert_pane(CmPane::RenameRules);
    let max_name_length_id = tiles.insert_pane(CmPane::MaxNameLength);
    let output_preview_id = tiles.insert_pane(CmPane::OutputPreview);
    let input_image_preview_id = tiles.insert_pane(CmPane::InputImagePreview);
    let threshold_preview_id = tiles.insert_pane(CmPane::ThresholdPreview);
    let output_image_preview_id = tiles.insert_pane(CmPane::OutputImagePreview);
    let image_description_id = tiles.insert_pane(CmPane::ImageDescription);
    let plan_id = include_plan.then(|| tiles.insert_pane(CmPane::Plan));
    let studio_guide_id = include_plan.then(|| tiles.insert_pane(CmPane::StudioGuide));

    // Left column: Input Paths + Input Images (vertical)
    let mut left_panes = Vec::new();
    if let Some(studio_guide_id) = studio_guide_id {
        left_panes.push(studio_guide_id);
    }
    left_panes.extend([input_paths_id, input_images_id]);
    let left_column = tiles.insert_vertical_tile(left_panes);

    // Middle-left column: Image previews stacked vertically (input, threshold, output)
    let previews_column = tiles.insert_vertical_tile(vec![
        input_image_preview_id,
        threshold_preview_id,
        output_image_preview_id,
    ]);

    // Middle column: Settings (Image Manipulation + Rename Rules + Max Name Length + Image Description)
    let mut settings_panes = vec![
        image_manipulation_id,
        rename_rules_id,
        max_name_length_id,
        image_description_id,
    ];
    if let Some(plan_id) = plan_id {
        settings_panes.push(plan_id);
    }
    let settings_column = tiles.insert_vertical_tile(settings_panes);

    // Right column: Output Preview
    let right_column = output_preview_id;

    // Main horizontal layout
    let root = tiles.insert_horizontal_tile(vec![
        left_column,
        previews_column,
        settings_column,
        right_column,
    ]);

    egui_tiles::Tree::new("cm_tree", root, tiles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const CORE_STUDIO_PANES: [&str; 10] = [
        "InputPaths",
        "InputImages",
        "ImageManipulation",
        "RenameRules",
        "MaxNameLength",
        "OutputPreview",
        "InputImagePreview",
        "ThresholdPreview",
        "OutputImagePreview",
        "ImageDescription",
    ];

    #[test]
    fn v1_studio_contains_core_panes_without_product_search() {
        let panes = pane_keys(&create_default_tree());

        assert_eq!(panes, BTreeSet::from(CORE_STUDIO_PANES));
    }

    #[test]
    fn v2_studio_extends_v1_with_guide_and_plan_only() {
        let mut expected = BTreeSet::from(CORE_STUDIO_PANES);
        expected.insert("Plan");
        expected.insert("StudioGuide");

        let panes = pane_keys(&create_v2_tree());

        assert_eq!(panes, expected);
    }

    #[test]
    fn product_search_is_a_standalone_mode() {
        let panes = pane_keys(&create_product_search_tree());

        assert_eq!(panes, BTreeSet::from(["ProductSearch"]));
    }

    fn pane_keys(tree: &egui_tiles::Tree<CmPane>) -> BTreeSet<&'static str> {
        let mut out = BTreeSet::new();
        if let Some(root) = tree.root() {
            collect_panes(tree, root, &mut out);
        }
        out
    }

    fn collect_panes(
        tree: &egui_tiles::Tree<CmPane>,
        tile_id: egui_tiles::TileId,
        out: &mut BTreeSet<&'static str>,
    ) {
        match tree.tiles.get(tile_id) {
            Some(egui_tiles::Tile::Pane(pane)) => {
                out.insert(pane.to_key());
            }
            Some(egui_tiles::Tile::Container(container)) => {
                for child in container.children() {
                    collect_panes(tree, *child, out);
                }
            }
            None => {}
        }
    }
}
