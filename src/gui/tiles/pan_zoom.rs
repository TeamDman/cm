//! Pan and zoom functionality for image previews

use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, Vec2};

/// State for pan and zoom on an image preview
#[derive(Clone, Debug, Default)]
pub struct PanZoomState {
    /// Zoom multiplier relative to fit-to-window (1.0 = fit perfectly)
    pub zoom_multiplier: f32,
    /// Offset in screen pixels
    pub offset: Vec2,
    /// Whether the state has been initialized
    initialized: bool,
    /// The calculated fit scale for this image (stored for reference)
    pub fit_scale: f32,
    /// Whether the user interacted with this preview this frame
    pub dirty: bool,
}

impl PanZoomState {
    pub fn new() -> Self {
        Self {
            zoom_multiplier: 1.0,
            offset: Vec2::ZERO,
            initialized: false,
            fit_scale: 1.0,
            dirty: false,
        }
    }
    
    /// Reset to fit the image in the available space
    pub fn reset(&mut self) {
        self.zoom_multiplier = 1.0;
        self.offset = Vec2::ZERO;
        self.initialized = false;
        self.dirty = true;
    }
    
    /// Get the actual zoom factor (fit_scale * zoom_multiplier)
    pub fn actual_zoom(&self) -> f32 {
        self.fit_scale * self.zoom_multiplier
    }
    
    /// Sync from another pan/zoom state (copies multiplier and offset, not fit_scale)
    pub fn sync_from(&mut self, other: &PanZoomState) {
        self.zoom_multiplier = other.zoom_multiplier;
        self.offset = other.offset;
    }
}

/// Draw a texture with pan and zoom support
pub fn draw_pan_zoom_image(
    ui: &mut egui::Ui,
    texture: &TextureHandle,
    state: &mut PanZoomState,
    _id_salt: &str,
) {
    let available = ui.available_size();
    let tex_size = texture.size_vec2();
    
    // Calculate fit scale for this image
    let fit_scale = (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
    state.fit_scale = fit_scale;
    
    // Initialize zoom multiplier if needed
    if !state.initialized {
        state.zoom_multiplier = 1.0;
        state.offset = Vec2::ZERO;
        state.initialized = true;
    }
    
    // Calculate the display size based on actual zoom
    let actual_zoom = state.actual_zoom();
    let display_size = tex_size * actual_zoom;
    
    // Allocate space for the image area
    let (rect, response) = ui.allocate_exact_size(
        available,
        egui::Sense::click_and_drag(),
    );
    
    // Handle panning with drag
    if response.dragged() {
        state.offset += response.drag_delta();
        state.dirty = true;
    }
    
    // Handle zooming with scroll wheel
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let old_zoom = actual_zoom;
            let zoom_factor = 1.0 + scroll * 0.001;
            state.zoom_multiplier = (state.zoom_multiplier * zoom_factor).clamp(0.1, 10.0);
            let new_zoom = state.actual_zoom();
            
            // Zoom towards the mouse position
            if let Some(hover_pos) = response.hover_pos() {
                let mouse_rel = hover_pos - rect.center();
                let scale_change = new_zoom / old_zoom;
                state.offset = state.offset * scale_change + mouse_rel * (1.0 - scale_change);
            }
            state.dirty = true;
        }
    }
    
    // Double-click or right-click to reset view
    if response.double_clicked() || response.secondary_clicked() {
        state.zoom_multiplier = 1.0;
        state.offset = Vec2::ZERO;
        state.dirty = true;
    }
    
    // Calculate image position (centered with offset)
    let image_center = rect.center() + state.offset;
    let image_rect = Rect::from_center_size(image_center, display_size);
    
    // Use a clipped painter to respect tile boundaries
    let painter = ui.painter().with_clip_rect(rect);
    
    // Calculate UV coordinates for the visible portion
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    
    painter.image(
        texture.id(),
        image_rect,
        uv,
        Color32::WHITE,
    );
    
    // Draw a subtle border around the view area
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, Color32::from_gray(60)),
        egui::epaint::StrokeKind::Inside,
    );
    
    // Show zoom level hint on hover
    if response.hovered() {
        let zoom_percent = (actual_zoom * 100.0).round() as i32;
        response.on_hover_text(format!("{}% - Scroll to zoom, drag to pan, double-click to reset", zoom_percent));
    }
}

/// Draw an image from a URI with pan and zoom support
pub fn draw_pan_zoom_image_uri(
    ui: &mut egui::Ui,
    uri: &str,
    state: &mut PanZoomState,
    _id_salt: &str,
) {
    let available = ui.available_size();
    
    // Try to get the actual image size for proper centering
    let image = egui::Image::new(uri);
    let image_size = image.calc_size(available, image.size());
    
    // Calculate fit scale for this image
    if image_size.x > 0.0 && image_size.y > 0.0 {
        let fit_scale = (available.x / image_size.x).min(available.y / image_size.y).min(1.0);
        state.fit_scale = fit_scale;
    }
    
    // Allocate space for the image area
    let (rect, response) = ui.allocate_exact_size(
        available,
        egui::Sense::click_and_drag(),
    );
    
    // Handle panning with drag
    if response.dragged() {
        state.offset += response.drag_delta();
        state.dirty = true;
    }
    
    // Handle zooming with scroll wheel
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let old_zoom = state.actual_zoom();
            let zoom_factor = 1.0 + scroll * 0.001;
            state.zoom_multiplier = (state.zoom_multiplier * zoom_factor).clamp(0.1, 10.0);
            let new_zoom = state.actual_zoom();
            
            // Zoom towards the mouse position
            if let Some(hover_pos) = response.hover_pos() {
                let mouse_rel = hover_pos - rect.center();
                let scale_change = new_zoom / old_zoom;
                state.offset = state.offset * scale_change + mouse_rel * (1.0 - scale_change);
            }
            state.dirty = true;
        }
    }
    
    // Double-click or right-click to reset view
    if response.double_clicked() || response.secondary_clicked() {
        state.zoom_multiplier = 1.0;
        state.offset = Vec2::ZERO;
        state.dirty = true;
    }
    
    // Initialize zoom multiplier if not set
    if !state.initialized {
        state.zoom_multiplier = 1.0;
        state.initialized = true;
    }
    
    // Calculate display size based on actual image dimensions and zoom
    let actual_zoom = state.actual_zoom();
    let display_size = if image_size.x > 0.0 && image_size.y > 0.0 {
        image_size * actual_zoom
    } else {
        available * state.zoom_multiplier
    };
    
    // Calculate image position (centered with offset)
    let image_center = rect.center() + state.offset;
    let image_rect = Rect::from_center_size(image_center, display_size);
    
    // For URI images, use a child UI with proper clipping
    {
        // Create a clipped child UI that respects the tile boundaries
        let mut image_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
        );
        image_ui.set_clip_rect(rect);
        
        // Position the image within the clipped area
        let image = egui::Image::new(uri);
        
        image_ui.allocate_new_ui(egui::UiBuilder::new().max_rect(image_rect), |ui| {
            ui.set_clip_rect(rect);
            ui.add(
                image
                    .fit_to_exact_size(display_size)
                    .maintain_aspect_ratio(true)
            );
        });
    }
    
    // Draw a subtle border around the view area (get painter after child UI is done)
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, Color32::from_gray(60)),
        egui::epaint::StrokeKind::Inside,
    );
    
    // Show zoom level hint on hover
    if response.hovered() {
        let zoom_percent = (actual_zoom * 100.0).round() as i32;
        response.on_hover_text(format!("{}% - Scroll to zoom, drag to pan, double-click to reset", zoom_percent));
    }
}
