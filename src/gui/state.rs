//! Shared application state for the CM GUI

use crate::app_home::APP_HOME;
use crate::image_processing::{self, ProcessingSettings, get_output_path, BinarizationMode};
use crate::inputs;
use crate::rename_rules::RenameRule;
use crate::MAX_NAME_LENGTH;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use tracing::{info, warn};

/// Shared application state
pub struct AppState {
    /// Cached input paths (refreshed from disk)
    pub input_paths: Vec<PathBuf>,
    /// Cached image files (derived from inputs)
    pub image_files: Vec<PathBuf>,
    /// Path to remove (deferred action)
    pub path_to_remove: Option<PathBuf>,
    /// Whether to clear all inputs (deferred action)
    pub clear_all: bool,
    /// Last error message
    pub last_error: Option<String>,
    /// Cached rename rules
    pub rename_rules: Vec<RenameRule>,
    /// Cached renamed file paths (after applying rules)
    pub renamed_files: Vec<PathBuf>,
    /// Hash key for rename preview cache invalidation
    pub rename_preview_key: u64,
    /// Current max name length value
    pub max_name_length: usize,
    /// Whether the logs window/tile is visible
    pub logs_visible: bool,
    /// Whether the about window is open
    pub about_open: bool,
    /// Currently selected input file (the source of truth for preview)
    pub selected_input_file: Option<PathBuf>,
    /// Currently previewed input image path (derived from selected_input_file)
    pub input_preview_path: Option<PathBuf>,
    /// Currently previewed output image path (derived from selected_input_file)
    pub output_preview_path: Option<PathBuf>,
    /// Whether we've initialized
    pub initialized: bool,
    /// Image manipulation: crop images to content
    pub crop_to_content: bool,
    /// Threshold value for crop detection (0-255)
    pub crop_threshold: u8,
    /// Binarization preview mode ("keep_white" or "keep_black")
    pub binarization_mode: BinarizationMode,
    /// Thickness of the red bounding box in threshold preview (1-10)
    pub box_thickness: u8,
    /// Synchronize pan/zoom across all image previews
    pub sync_preview_pan_zoom: bool,
    /// Cached output info for the selected image
    pub selected_output_info: Option<OutputImageInfo>,
    /// Processing result message
    pub processing_result: Option<String>,
    /// Whether output info is being calculated in the background
    pub output_info_loading: bool,
    /// Whether process_all is running in the background
    pub process_all_running: bool,
    /// Progress for process_all (current, total)
    pub process_all_progress: Option<(usize, usize)>,
    /// Sender for background tasks
    background_sender: Sender<BackgroundMessage>,
    /// Receiver for background task results
    background_receiver: Receiver<BackgroundMessage>,
}

/// Info about a processed output image
#[derive(Clone, Debug)]
pub struct OutputImageInfo {
    pub estimated_size: u64,
    pub original_width: u32,
    pub original_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub was_cropped: bool,
    /// PNG bytes of the processed image (for preview)
    pub preview_data: Vec<u8>,
    /// PNG bytes of the binarized threshold preview
    pub threshold_preview_data: Vec<u8>,
    /// Crop bounds (x, y, width, height)
    pub crop_bounds: Option<(u32, u32, u32, u32)>,
}

/// Messages sent from background processing threads
pub enum BackgroundMessage {
    /// Output info for a selected image is ready
    OutputInfoReady {
        input_path: PathBuf,
        info: OutputImageInfo,
    },
    /// Output info processing failed
    OutputInfoError {
        input_path: PathBuf,
        error: String,
    },
    /// Processing all images completed
    ProcessAllComplete {
        processed_count: usize,
        error_count: usize,
        errors: Vec<String>,
    },
    /// Progress update for processing all images
    ProcessAllProgress {
        current: usize,
        total: usize,
        current_file: PathBuf,
    },
}

impl Default for AppState {
    fn default() -> Self {
        let (background_sender, background_receiver) = mpsc::channel();
        Self {
            input_paths: Vec::new(),
            image_files: Vec::new(),
            path_to_remove: None,
            clear_all: false,
            last_error: None,
            rename_rules: Vec::new(),
            renamed_files: Vec::new(),
            rename_preview_key: 0,
            max_name_length: MAX_NAME_LENGTH.load(Ordering::SeqCst),
            logs_visible: false,
            about_open: false,
            selected_input_file: None,
            input_preview_path: None,
            output_preview_path: None,
            initialized: false,
            crop_to_content: true,
            crop_threshold: 10,
            binarization_mode: BinarizationMode::KeepWhite,
            box_thickness: 10,
            sync_preview_pan_zoom: true,
            selected_output_info: None,
            processing_result: None,
            output_info_loading: false,
            process_all_running: false,
            process_all_progress: None,
            background_sender,
            background_receiver,
        }
    }
}


impl AppState {
    /// Reload all data from disk
    pub fn reload_data(&mut self) {
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

        // Load rename rules
        match crate::rename_rules::list_rules(&APP_HOME) {
            Ok(rules) => {
                self.rename_rules = rules.into_iter().map(|(_, r)| r).collect();
            }
            Err(e) => {
                if self.last_error.is_none() {
                    self.last_error = Some(format!("Failed to load rules: {}", e));
                }
                self.rename_rules.clear();
            }
        }

        // Update max name length
        self.max_name_length = MAX_NAME_LENGTH.load(Ordering::SeqCst);

        // Invalidate rename preview cache
        self.rename_preview_key = 0;
    }

    /// Handle deferred actions from previous frame
    pub fn handle_deferred_actions(&mut self) {
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

    /// Update the renamed files cache if needed
    pub fn update_rename_preview(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.image_files.len().hash(&mut hasher);
        self.max_name_length.hash(&mut hasher);
        for r in &self.rename_rules {
            r.id.hash(&mut hasher);
            r.find.hash(&mut hasher);
            r.replace.hash(&mut hasher);
            r.enabled.hash(&mut hasher);
            r.case_sensitive.hash(&mut hasher);
            r.only_when_name_too_long.hash(&mut hasher);
        }
        let key = hasher.finish();

        if self.rename_preview_key != key {
            self.renamed_files = apply_rules_seq(&self.image_files, &self.rename_rules, self.max_name_length);
            self.rename_preview_key = key;
        }
    }

    /// Select an input file and update both previews
    pub fn select_file(&mut self, input_path: PathBuf) {
        // First ensure renamed_files is up to date
        self.update_rename_preview();
        
        self.selected_input_file = Some(input_path.clone());
        self.input_preview_path = Some(input_path.clone());
        
        // Find the corresponding output path
        if let Some(idx) = self.image_files.iter().position(|p| p == &input_path) {
            if let Some(renamed) = self.renamed_files.get(idx) {
                // Find which input root this belongs to
                if let Some(input_root) = self.input_paths.iter().find(|r| input_path.starts_with(r)) {
                    let renamed_name = renamed
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    
                    if let Some(output_path) = get_output_path(&input_path, input_root, &renamed_name) {
                        self.output_preview_path = Some(output_path);
                    }
                }
            }
        }
        
        // Update output info (process the image to get size/dimensions)
        self.update_selected_output_info();
    }

    /// Update the output info for the selected file (runs in background thread)
    pub fn update_selected_output_info(&mut self) {
        let Some(ref input_path) = self.selected_input_file else {
            self.selected_output_info = None;
            self.output_info_loading = false;
            return;
        };
        
        // Mark as loading
        self.output_info_loading = true;
        self.selected_output_info = None;
        
        let settings = ProcessingSettings {
            crop_to_content: self.crop_to_content,
            crop_threshold: self.crop_threshold,
            binarization_mode: self.binarization_mode,
            box_thickness: self.box_thickness,
        };
        let input_path = input_path.clone();
        let sender = self.background_sender.clone();
        
        thread::spawn(move || {
            match image_processing::process_image(&input_path, &settings) {
                Ok(processed) => {
                    let info = OutputImageInfo {
                        estimated_size: processed.estimated_size,
                        original_width: processed.original_width,
                        original_height: processed.original_height,
                        output_width: processed.output_width,
                        output_height: processed.output_height,
                        was_cropped: processed.was_cropped,
                        preview_data: processed.data,
                        threshold_preview_data: processed.threshold_preview_data,
                        crop_bounds: processed.crop_bounds,
                    };
                    let _ = sender.send(BackgroundMessage::OutputInfoReady {
                        input_path,
                        info,
                    });
                }
                Err(e) => {
                    let _ = sender.send(BackgroundMessage::OutputInfoError {
                        input_path,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Process all images according to current settings (runs in background thread)
    pub fn process_all(&mut self) {
        if self.process_all_running {
            warn!("Process all already running, ignoring request");
            return;
        }
        
        self.update_rename_preview();
        
        let settings = ProcessingSettings {
            crop_to_content: self.crop_to_content,
            crop_threshold: self.crop_threshold,
            binarization_mode: self.binarization_mode,
            box_thickness: self.box_thickness,
        };
        
        let image_files = self.image_files.clone();
        let renamed_files = self.renamed_files.clone();
        let input_paths = self.input_paths.clone();
        let sender = self.background_sender.clone();
        
        self.process_all_running = true;
        self.process_all_progress = Some((0, image_files.len()));
        self.processing_result = None;
        
        thread::spawn(move || {
            match image_processing::process_all_images(
                &image_files,
                &renamed_files,
                &input_paths,
                &settings,
                None,
            ) {
                Ok(result) => {
                    let _ = sender.send(BackgroundMessage::ProcessAllComplete {
                        processed_count: result.processed_count,
                        error_count: result.error_count,
                        errors: result.errors,
                    });
                }
                Err(e) => {
                    let _ = sender.send(BackgroundMessage::ProcessAllComplete {
                        processed_count: 0,
                        error_count: 1,
                        errors: vec![e.to_string()],
                    });
                }
            }
        });
    }
    
    /// Poll for background task completions (call this each frame)
    pub fn poll_background_tasks(&mut self) {
        // Process all pending messages
        while let Ok(msg) = self.background_receiver.try_recv() {
            match msg {
                BackgroundMessage::OutputInfoReady { input_path, info } => {
                    // Only update if this is still the selected file
                    if self.selected_input_file.as_ref() == Some(&input_path) {
                        self.selected_output_info = Some(info);
                        self.output_info_loading = false;
                    }
                }
                BackgroundMessage::OutputInfoError { input_path, error } => {
                    if self.selected_input_file.as_ref() == Some(&input_path) {
                        self.output_info_loading = false;
                        info!("Failed to process image {}: {}", input_path.display(), error);
                    }
                }
                BackgroundMessage::ProcessAllComplete { processed_count, error_count, errors } => {
                    self.process_all_running = false;
                    self.process_all_progress = None;
                    self.processing_result = Some(format!(
                        "Processed {} files. {} errors.",
                        processed_count,
                        error_count
                    ));
                    if !errors.is_empty() {
                        info!("Processing errors: {:?}", errors);
                    }
                }
                BackgroundMessage::ProcessAllProgress { current, total, current_file: _ } => {
                    self.process_all_progress = Some((current, total));
                }
            }
        }
    }
}

/// Check if a path is an image file
pub fn is_image_file(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff"
        )
    } else {
        false
    }
}

/// Apply rename rules sequentially to file base names
fn apply_rules_seq(files: &[PathBuf], rules: &[RenameRule], max_name_length: usize) -> Vec<PathBuf> {
    // Precompile regexes once per rule
    let compiled: Vec<Option<regex::Regex>> = rules
        .iter()
        .map(|r| {
            let mut builder = regex::RegexBuilder::new(&r.find);
            if !r.case_sensitive {
                builder.case_insensitive(true);
            }
            builder.build().ok()
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
                // Skip disabled rules
                if !rule.enabled {
                    continue;
                }

                // Check if rule only applies when name is too long
                if rule.only_when_name_too_long && cur.len() <= max_name_length {
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
