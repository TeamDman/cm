//! Tile behavior and pane definitions for egui_tiles

use crate::gui::state::AppState;
use crate::gui::tiles;
use eframe::egui;

/// The different types of panes in our application
#[derive(Clone, Debug)]
pub enum CmPane {
    /// Input paths management
    InputPaths,
    /// Tree view of input images
    InputImages,
    /// Rename rules management
    RenameRules,
    /// Max name length setting
    MaxNameLength,
    /// Output preview with renamed files
    OutputPreview,
    /// Input image preview
    InputImagePreview,
    /// Output image preview  
    OutputImagePreview,
    /// Logs viewer
    Logs,
}

impl CmPane {
    /// Get the title for this pane
    pub fn title(&self) -> &'static str {
        match self {
            CmPane::InputPaths => "Input Paths",
            CmPane::InputImages => "Input Images",
            CmPane::RenameRules => "Rename Rules",
            CmPane::MaxNameLength => "Max Name Length",
            CmPane::OutputPreview => "Output Preview",
            CmPane::InputImagePreview => "Input Preview",
            CmPane::OutputImagePreview => "Output Preview Image",
            CmPane::Logs => "Logs",
        }
    }
}

/// Behavior implementation for our tile tree
pub struct CmBehavior<'a> {
    pub state: &'a mut AppState,
}

impl<'a> egui_tiles::Behavior<CmPane> for CmBehavior<'a> {
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
            CmPane::InputImages => tiles::draw_input_images_tile(ui, self.state),
            CmPane::RenameRules => tiles::draw_rename_rules_tile(ui, self.state),
            CmPane::MaxNameLength => tiles::draw_max_name_length_tile(ui, self.state),
            CmPane::OutputPreview => tiles::draw_output_preview_tile(ui, self.state),
            CmPane::InputImagePreview => tiles::draw_input_image_preview_tile(ui, self.state),
            CmPane::OutputImagePreview => tiles::draw_output_image_preview_tile(ui, self.state),
            CmPane::Logs => tiles::draw_logs_tile(ui),
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
    let rename_rules_id = tiles.insert_pane(CmPane::RenameRules);
    let max_name_length_id = tiles.insert_pane(CmPane::MaxNameLength);
    let output_preview_id = tiles.insert_pane(CmPane::OutputPreview);

    // Left column: Input Paths + Input Images (vertical)
    let left_column = tiles.insert_vertical_tile(vec![input_paths_id, input_images_id]);

    // Middle column: Rename Rules + Max Name Length (vertical)
    let middle_column = tiles.insert_vertical_tile(vec![rename_rules_id, max_name_length_id]);

    // Right column: Output Preview
    let right_column = output_preview_id;

    // Main horizontal layout
    let root = tiles.insert_horizontal_tile(vec![left_column, middle_column, right_column]);

    egui_tiles::Tree::new("cm_tree", root, tiles)
}
