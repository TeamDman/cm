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
    let product_search_id = tiles.insert_pane(CmPane::ProductSearch);

    // Left column: Input Paths + Input Images (vertical)
    let left_column = tiles.insert_vertical_tile(vec![input_paths_id, input_images_id]);

    // Middle-left column: Image previews stacked vertically (input, threshold, output)
    let previews_column = tiles.insert_vertical_tile(vec![
        input_image_preview_id,
        threshold_preview_id,
        output_image_preview_id,
    ]);

    // Middle column: Settings (Image Manipulation + Rename Rules + Max Name Length + Image Description + Product Search)
    let settings_column = tiles.insert_vertical_tile(vec![
        image_manipulation_id,
        rename_rules_id,
        max_name_length_id,
        image_description_id,
        product_search_id,
    ]);

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
