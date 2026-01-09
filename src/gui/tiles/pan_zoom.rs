//! Pan and zoom functionality for image previews

use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, Vec2};

/// State for pan and zoom on an image preview
#[derive(Clone, Debug, Default)]
pub struct PanZoomState {
    /// Zoom factor (1.0 = 100%)
    pub zoom: f32,
    /// Offset in screen pixels
    pub offset: Vec2,
    /// Whether the state has been initialized
    initialized: bool,
}

impl PanZoomState {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            offset: Vec2::ZERO,
            initialized: false,
        }
    }
    
    /// Reset to fit the image in the available space
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.offset = Vec2::ZERO;
        self.initialized = false;
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
    
    // Initialize zoom to fit image in available space
    if !state.initialized {
        let fit_scale = (available.x / tex_size.x).min(available.y / tex_size.y);
        state.zoom = fit_scale.min(1.0); // Start at fit or 100%, whichever is smaller
        state.offset = Vec2::ZERO;
        state.initialized = true;
    }
    
    // Calculate the display size based on zoom
    let display_size = tex_size * state.zoom;
    
    // Allocate space for the image area
    let (rect, response) = ui.allocate_exact_size(
        available,
        egui::Sense::click_and_drag(),
    );
    
    // Handle panning with drag
    if response.dragged() {
        state.offset += response.drag_delta();
    }
    
    // Handle zooming with scroll wheel
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let old_zoom = state.zoom;
            let zoom_factor = 1.0 + scroll * 0.001;
            state.zoom = (state.zoom * zoom_factor).clamp(0.1, 10.0);
            
            // Zoom towards the mouse position
            if let Some(hover_pos) = response.hover_pos() {
                let mouse_rel = hover_pos - rect.center();
                let scale_change = state.zoom / old_zoom;
                state.offset = state.offset * scale_change + mouse_rel * (1.0 - scale_change);
            }
        }
    }
    
    // Double-click or right-click to reset view
    if response.double_clicked() || response.secondary_clicked() {
        state.zoom = 1.0;
        state.offset = Vec2::ZERO;
        state.initialized = false;
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
        let zoom_percent = (state.zoom * 100.0).round() as i32;
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
    
    // Allocate space for the image area
    let (rect, response) = ui.allocate_exact_size(
        available,
        egui::Sense::click_and_drag(),
    );
    
    // Handle panning with drag
    if response.dragged() {
        state.offset += response.drag_delta();
    }
    
    // Handle zooming with scroll wheel
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let old_zoom = state.zoom;
            let zoom_factor = 1.0 + scroll * 0.001;
            state.zoom = (state.zoom * zoom_factor).clamp(0.1, 10.0);
            
            // Zoom towards the mouse position
            if let Some(hover_pos) = response.hover_pos() {
                let mouse_rel = hover_pos - rect.center();
                let scale_change = state.zoom / old_zoom;
                state.offset = state.offset * scale_change + mouse_rel * (1.0 - scale_change);
            }
        }
    }
    
    // Double-click or right-click to reset view
    if response.double_clicked() || response.secondary_clicked() {
        state.zoom = 1.0;
        state.offset = Vec2::ZERO;
        state.initialized = false;
    }
    
    // Initialize zoom to fit image in available space (like draw_pan_zoom_image)
    if !state.initialized && image_size.x > 0.0 && image_size.y > 0.0 {
        let fit_scale = (available.x / image_size.x).min(available.y / image_size.y);
        state.zoom = fit_scale.min(1.0); // Start at fit or 100%, whichever is smaller
        state.offset = Vec2::ZERO;
        state.initialized = true;
    }
    
    // Calculate display size based on actual image dimensions and zoom
    let display_size = if image_size.x > 0.0 && image_size.y > 0.0 {
        image_size * state.zoom
    } else {
        available * state.zoom
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
        let zoom_percent = (state.zoom * 100.0).round() as i32;
        response.on_hover_text(format!("{}% - Scroll to zoom, drag to pan, double-click to reset", zoom_percent));
    }
}
