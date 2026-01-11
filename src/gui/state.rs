//! Shared application state for the CM GUI

use crate::MAX_NAME_LENGTH;
use crate::app_home::APP_HOME;
use crate::cli::command::search::search_result_ok::SearchResultOk;
use crate::image_processing::BinarizationMode;
use crate::image_processing::ProcessingSettings;
use crate::image_processing::get_output_path;
use crate::image_processing::{self};
use crate::inputs;
use crate::rename_rules::RenameRule;
use chrono::DateTime;
use chrono::Local;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::{self};
use tracing::error;
use tracing::info;
use tracing::warn;

/// Thumbnail size for cached previews
pub const THUMBNAIL_SIZE: u32 = 128;

/// Cached image metadata and thumbnail
#[derive(Clone, Debug)]
pub struct CachedImageInfo {
    /// Image width
    pub width: u32,
    /// Image height  
    pub height: u32,
    /// File size in bytes
    pub file_size: u64,
    /// Thumbnail PNG data (small, for tooltips)
    pub thumbnail_data: Vec<u8>,
}

/// Loading state for async operations
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LoadingState {
    #[default]
    NotStarted,
    Loading,
    Loaded,
    Failed(String),
}

impl LoadingState {
    pub fn is_loading(&self) -> bool {
        matches!(self, LoadingState::Loading)
    }
}

/// Shared application state
pub struct AppState {
    /// Cached input paths (refreshed from disk)
    pub input_paths: Vec<PathBuf>,
    /// Loading state for input paths
    pub input_paths_loading: LoadingState,
    /// Cached image files (derived from inputs)
    pub image_files: Vec<PathBuf>,
    /// Loading state for image file discovery
    pub image_files_loading: LoadingState,
    /// Path to remove (deferred action)
    pub path_to_remove: Option<PathBuf>,
    /// Whether to clear all inputs (deferred action)
    pub clear_all: bool,
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
    /// JPEG output quality (1-100)
    pub jpeg_quality: u8,
    /// Cached output info for the selected image
    pub selected_output_info: Option<OutputImageInfo>,
    /// Whether output info is being calculated in the background
    pub output_info_loading: bool,
    /// Whether process_all is running in the background
    pub process_all_running: bool,
    /// Progress for process_all (current, total)
    pub process_all_progress: Option<(usize, usize)>,
    /// Cache of image metadata and thumbnails (path -> info)
    pub image_cache: HashMap<PathBuf, CachedImageInfo>,
    /// Set of paths currently being loaded in background
    pub images_loading: HashSet<PathBuf>,
    /// Product search tile: query string
    pub product_search_query: String,
    /// Product search tile: SKU string
    pub product_search_sku: String,
    /// Whether to auto-populate query/sku from the suggested values
    pub product_search_use_suggestion: bool,
    /// Product search tile: parsed search result (struct), if any
    pub product_search_result_raw: Option<SearchResultOk>,
    /// Product search tile: result JSON (pretty-printed) stored to avoid re-prettifying
    pub product_search_result_pretty: String,
    /// When the last response was received (if any)
    pub product_search_last_response: Option<DateTime<Local>>,
    /// Whether the raw pretty JSON is expanded
    pub product_search_show_raw: bool,
    /// Sender for background tasks
    pub background_sender: UnboundedSender<BackgroundMessage>,
    /// Receiver for background task results
    background_receiver: UnboundedReceiver<BackgroundMessage>,
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
    /// Downsampled PNG bytes of the processed image (for GUI preview)
    pub preview_data: Vec<u8>,
    /// PNG bytes of the binarized threshold preview (downsampled)
    pub threshold_preview_data: Vec<u8>,
    /// Crop bounds (x, y, width, height)
    pub crop_bounds: Option<(u32, u32, u32, u32)>,
}

/// Messages sent from background processing threads
#[derive(Debug)]
pub enum BackgroundMessage {
    /// Input paths loaded
    InputPathsReady { paths: Vec<PathBuf> },
    /// Input paths loading failed
    InputPathsError { error: String },
    /// Image files discovered
    ImageFilesReady { files: Vec<PathBuf> },
    /// Image files discovery failed
    ImageFilesError { error: String },
    /// Output info for a selected image is ready
    OutputInfoReady {
        input_path: PathBuf,
        info: OutputImageInfo,
    },
    /// Output info processing failed
    OutputInfoError { input_path: PathBuf, error: String },
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
    /// Image cache entry loaded
    ImageCacheReady {
        path: PathBuf,
        info: CachedImageInfo,
    },
    /// Image cache loading failed
    ImageCacheError { path: PathBuf },
    /// Processing a single selected image completed
    ProcessSelectedComplete {
        success: bool,
        error: Option<String>,
    },
    /// Product search result (parsed struct and prettified JSON) from Searchspring
    ProductSearchResult {
        result: Option<SearchResultOk>,
        pretty: Option<String>,
        error: Option<String>,
        /// When the response was received on the background thread
        received_at: DateTime<Local>,
    },
}

impl Default for AppState {
    fn default() -> Self {
        let (background_sender, background_receiver) = mpsc::unbounded_channel();
        Self {
            input_paths: Vec::new(),
            input_paths_loading: LoadingState::NotStarted,
            image_files: Vec::new(),
            image_files_loading: LoadingState::NotStarted,
            path_to_remove: None,
            clear_all: false,
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
            jpeg_quality: 90,
            selected_output_info: None,
            output_info_loading: false,
            process_all_running: false,
            process_all_progress: None,
            image_cache: HashMap::new(),
            images_loading: HashSet::new(),
            product_search_query: String::new(),
            product_search_sku: String::new(),
            product_search_use_suggestion: true,
            product_search_result_raw: None,
            product_search_result_pretty: String::new(),
            product_search_last_response: None,
            product_search_show_raw: false,
            background_sender,
            background_receiver,
        }
    }
}

impl AppState {
    /// Start async reload of all data - does NOT block!
    pub fn reload_data(&mut self) {
        // Start loading input paths in background
        self.start_load_input_paths();

        // Load rename rules (these are small, can stay sync for now)
        match crate::rename_rules::list_rules(&APP_HOME) {
            Ok(rules) => {
                self.rename_rules = rules.into_iter().map(|(_, r)| r).collect();
            }
            Err(e) => {
                error!("Failed to load rename rules: {}", e);
                self.rename_rules.clear();
            }
        }

        // Update max name length
        self.max_name_length = MAX_NAME_LENGTH.load(Ordering::SeqCst);

        // Invalidate rename preview cache
        self.rename_preview_key = 0;
    }

    /// Start loading input paths in background
    fn start_load_input_paths(&mut self) {
        self.input_paths_loading = LoadingState::Loading;
        let sender = self.background_sender.clone();

        tokio::spawn(async move {
            // Use spawn_blocking for the file I/O
            let result = tokio::task::spawn_blocking(|| inputs::load_inputs(&APP_HOME)).await;

            match result {
                Ok(Ok(paths)) => {
                    let _ = sender.send(BackgroundMessage::InputPathsReady { paths });
                }
                Ok(Err(e)) => {
                    let _ = sender.send(BackgroundMessage::InputPathsError {
                        error: e.to_string(),
                    });
                }
                Err(e) => {
                    let _ = sender.send(BackgroundMessage::InputPathsError {
                        error: format!("Task panicked: {}", e),
                    });
                }
            }
        });
    }

    /// Start discovering image files in background
    fn start_discover_image_files(&mut self) {
        self.image_files_loading = LoadingState::Loading;
        let sender = self.background_sender.clone();

        tokio::spawn(async move {
            // Use spawn_blocking for the recursive directory walk
            let result = tokio::task::spawn_blocking(|| inputs::list_files(&APP_HOME)).await;

            match result {
                Ok(Ok(files)) => {
                    // Filter to image files
                    let image_files: Vec<PathBuf> = files
                        .into_iter()
                        .filter(|p| is_image_file(p.as_path()))
                        .collect();
                    let _ = sender.send(BackgroundMessage::ImageFilesReady { files: image_files });
                }
                Ok(Err(e)) => {
                    let _ = sender.send(BackgroundMessage::ImageFilesError {
                        error: e.to_string(),
                    });
                }
                Err(e) => {
                    let _ = sender.send(BackgroundMessage::ImageFilesError {
                        error: format!("Task panicked: {}", e),
                    });
                }
            }
        });
    }

    /// Start background loading for all images not yet in cache
    /// Uses a single background task that processes images with limited concurrency
    pub fn start_image_cache_loading(&mut self) {
        // Collect paths that need loading
        let paths_to_load: Vec<PathBuf> = self
            .image_files
            .iter()
            .filter(|p| !self.image_cache.contains_key(*p) && !self.images_loading.contains(*p))
            .cloned()
            .collect();

        if paths_to_load.is_empty() {
            return;
        }

        // Mark all as loading
        for path in &paths_to_load {
            self.images_loading.insert(path.clone());
        }

        let sender = self.background_sender.clone();

        // Spawn a single task that processes images with concurrency limit
        tokio::spawn(async move {
            // Process images with limited concurrency (4 at a time)
            let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(4));

            let mut handles = Vec::new();

            for path in paths_to_load {
                let sender = sender.clone();
                let semaphore = semaphore.clone();

                let handle = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await;

                    let path_clone = path.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        image_processing::load_image_metadata(&path_clone, THUMBNAIL_SIZE)
                    })
                    .await;

                    match result {
                        Ok(Ok(info)) => {
                            let _ = sender.send(BackgroundMessage::ImageCacheReady { path, info });
                        }
                        _ => {
                            let _ = sender.send(BackgroundMessage::ImageCacheError { path });
                        }
                    }
                });

                handles.push(handle);
            }

            // Wait for all to complete
            for handle in handles {
                let _ = handle.await;
            }
        });
    }

    /// Check if an image is still loading
    pub fn is_image_loading(&self, path: &PathBuf) -> bool {
        self.images_loading.contains(path)
    }

    /// Get cached image info if available
    pub fn get_cached_image(&self, path: &PathBuf) -> Option<&CachedImageInfo> {
        self.image_cache.get(path)
    }

    /// Handle deferred actions from previous frame
    pub fn handle_deferred_actions(&mut self) {
        // Handle clear all
        if self.clear_all {
            self.clear_all = false;
            self.input_paths_loading = LoadingState::Loading;
            let sender = self.background_sender.clone();

            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(|| inputs::clear_all(&APP_HOME)).await;

                match result {
                    Ok(Ok(())) => {
                        info!("Cleared all inputs");
                        // Trigger reload by sending empty paths
                        let _ =
                            sender.send(BackgroundMessage::InputPathsReady { paths: Vec::new() });
                    }
                    Ok(Err(e)) => {
                        let _ = sender.send(BackgroundMessage::InputPathsError {
                            error: format!("Failed to clear: {}", e),
                        });
                    }
                    Err(e) => {
                        let _ = sender.send(BackgroundMessage::InputPathsError {
                            error: format!("Task panicked: {}", e),
                        });
                    }
                }
            });
        }

        // Handle single path removal
        if let Some(path) = self.path_to_remove.take() {
            self.input_paths_loading = LoadingState::Loading;
            let sender = self.background_sender.clone();

            tokio::spawn(async move {
                let path_clone = path.clone();
                let result = tokio::task::spawn_blocking(move || {
                    inputs::remove_path(&APP_HOME, &path_clone)
                })
                .await;

                match result {
                    Ok(Ok(removed)) => {
                        if removed {
                            info!("Removed input: {}", path.display());
                        }
                        // Trigger reload
                        match tokio::task::spawn_blocking(|| inputs::load_inputs(&APP_HOME)).await {
                            Ok(Ok(paths)) => {
                                let _ = sender.send(BackgroundMessage::InputPathsReady { paths });
                            }
                            _ => {
                                let _ = sender
                                    .send(BackgroundMessage::InputPathsReady { paths: Vec::new() });
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        let _ = sender.send(BackgroundMessage::InputPathsError {
                            error: format!("Failed to remove: {}", e),
                        });
                    }
                    Err(e) => {
                        let _ = sender.send(BackgroundMessage::InputPathsError {
                            error: format!("Task panicked: {}", e),
                        });
                    }
                }
            });
        }
    }

    /// Update the renamed files cache if needed
    pub fn update_rename_preview(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        use std::hash::Hasher;

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
            self.renamed_files =
                apply_rules_seq(&self.image_files, &self.rename_rules, self.max_name_length);
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
        if let Some(idx) = self.image_files.iter().position(|p| p == &input_path)
            && let Some(renamed) = self.renamed_files.get(idx)
        {
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

        // Update output info (process the image to get size/dimensions)
        self.update_selected_output_info();
    }

    /// Update the output info for the selected file (runs in background)
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
            jpeg_quality: self.jpeg_quality,
        };
        let input_path = input_path.clone();
        let sender = self.background_sender.clone();

        tokio::spawn(async move {
            let input_path_clone = input_path.clone();
            let result = tokio::task::spawn_blocking(move || {
                image_processing::process_image(&input_path_clone, &settings)
            })
            .await;

            match result {
                Ok(Ok(processed)) => {
                    let info = OutputImageInfo {
                        estimated_size: processed.estimated_size,
                        original_width: processed.original_width,
                        original_height: processed.original_height,
                        output_width: processed.output_width,
                        output_height: processed.output_height,
                        was_cropped: processed.was_cropped,
                        preview_data: processed.output_preview_data,
                        threshold_preview_data: processed.threshold_preview_data,
                        crop_bounds: processed.crop_bounds,
                    };
                    let _ = sender.send(BackgroundMessage::OutputInfoReady { input_path, info });
                }
                Ok(Err(e)) => {
                    let _ = sender.send(BackgroundMessage::OutputInfoError {
                        input_path,
                        error: e.to_string(),
                    });
                }
                Err(e) => {
                    let _ = sender.send(BackgroundMessage::OutputInfoError {
                        input_path,
                        error: format!("Task panicked: {}", e),
                    });
                }
            }
        });
    }

    /// Process all images according to current settings (runs in background)
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
            jpeg_quality: self.jpeg_quality,
        };

        let image_files = self.image_files.clone();
        let renamed_files = self.renamed_files.clone();
        let input_paths = self.input_paths.clone();
        let sender = self.background_sender.clone();

        self.process_all_running = true;
        self.process_all_progress = Some((0, image_files.len()));

        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                image_processing::process_all_images(
                    &image_files,
                    &renamed_files,
                    &input_paths,
                    &settings,
                    None,
                )
            })
            .await;

            match result {
                Ok(Ok(result)) => {
                    let _ = sender.send(BackgroundMessage::ProcessAllComplete {
                        processed_count: result.processed_count,
                        error_count: result.error_count,
                        errors: result.errors,
                    });
                }
                Ok(Err(e)) => {
                    let _ = sender.send(BackgroundMessage::ProcessAllComplete {
                        processed_count: 0,
                        error_count: 1,
                        errors: vec![e.to_string()],
                    });
                }
                Err(e) => {
                    let _ = sender.send(BackgroundMessage::ProcessAllComplete {
                        processed_count: 0,
                        error_count: 1,
                        errors: vec![format!("Task panicked: {}", e)],
                    });
                }
            }
        });
    }

    /// Process just the currently selected image (runs in background)
    pub fn process_selected(&mut self) {
        if self.process_all_running {
            warn!("Processing already running, ignoring request");
            return;
        }

        let Some(selected_input) = self.selected_input_file.clone() else {
            error!("No file selected");
            return;
        };

        // Find the corresponding renamed file
        let Some(idx) = self.image_files.iter().position(|f| f == &selected_input) else {
            error!("Selected file not found in image list");
            return;
        };

        let Some(renamed_file) = self.renamed_files.get(idx).cloned() else {
            error!("No renamed file for selection");
            return;
        };

        // Find input root
        let Some(input_root) = self
            .input_paths
            .iter()
            .find(|r| selected_input.starts_with(r))
            .cloned()
        else {
            error!("Could not find input root for selected file");
            return;
        };

        self.update_rename_preview();

        let settings = ProcessingSettings {
            crop_to_content: self.crop_to_content,
            crop_threshold: self.crop_threshold,
            binarization_mode: self.binarization_mode,
            box_thickness: self.box_thickness,
            jpeg_quality: self.jpeg_quality,
        };

        let sender = self.background_sender.clone();

        self.process_all_running = true;
        self.process_all_progress = Some((0, 1));

        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || -> eyre::Result<()> {
                // Get the renamed filename
                let renamed_name = renamed_file
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Calculate output path
                let Some(output_path) =
                    image_processing::get_output_path(&selected_input, &input_root, &renamed_name)
                else {
                    return Err(eyre::eyre!("Could not calculate output path"));
                };

                // Create output directory if needed
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Process the image
                let processed = image_processing::process_image(&selected_input, &settings)?;

                // Write output file
                std::fs::write(&output_path, &processed.data)?;

                Ok(())
            })
            .await;

            match result {
                Ok(Ok(())) => {
                    let _ = sender.send(BackgroundMessage::ProcessSelectedComplete {
                        success: true,
                        error: None,
                    });
                }
                Ok(Err(e)) => {
                    let _ = sender.send(BackgroundMessage::ProcessSelectedComplete {
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
                Err(e) => {
                    let _ = sender.send(BackgroundMessage::ProcessSelectedComplete {
                        success: false,
                        error: Some(format!("Task panicked: {}", e)),
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
                BackgroundMessage::InputPathsReady { paths } => {
                    self.input_paths = paths;
                    self.input_paths_loading = LoadingState::Loaded;
                    // Now start discovering image files
                    self.start_discover_image_files();
                }
                BackgroundMessage::InputPathsError { error } => {
                    self.input_paths_loading = LoadingState::Failed(error.clone());
                    error!("Failed to load inputs: {}", error);
                    self.input_paths.clear();
                }
                BackgroundMessage::ImageFilesReady { mut files } => {
                    files.sort();
                    self.image_files = files;
                    self.image_files_loading = LoadingState::Loaded;
                    // Now start loading image metadata in background
                    self.start_image_cache_loading();
                }
                BackgroundMessage::ImageFilesError { error } => {
                    self.image_files_loading = LoadingState::Failed(error.clone());
                    error!("Failed to list files: {}", error);
                    self.image_files.clear();
                }
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
                        error!(
                            "Failed to process image {}: {}",
                            input_path.display(),
                            error
                        );
                    }
                }
                BackgroundMessage::ProcessAllComplete {
                    processed_count,
                    error_count,
                    errors,
                } => {
                    self.process_all_running = false;
                    self.process_all_progress = None;
                    info!(
                        "Processing complete: {} files processed, {} errors",
                        processed_count, error_count
                    );
                    if !errors.is_empty() {
                        error!("Processing errors: {:?}", errors);
                    }
                }
                BackgroundMessage::ProcessAllProgress {
                    current,
                    total,
                    current_file: _,
                } => {
                    self.process_all_progress = Some((current, total));
                }
                BackgroundMessage::ImageCacheReady { path, info } => {
                    self.images_loading.remove(&path);
                    self.image_cache.insert(path, info);
                }
                BackgroundMessage::ImageCacheError { path } => {
                    self.images_loading.remove(&path);
                }
                BackgroundMessage::ProductSearchResult {
                    result,
                    pretty,
                    error,
                    received_at,
                } => {
                    // Record when we got the response so UI can show it
                    self.product_search_last_response = Some(received_at);

                    if let Some(err) = error {
                        error!("Product search failed: {}", err);
                        self.product_search_result_raw = None;
                        self.product_search_result_pretty.clear();
                    } else {
                        self.product_search_result_raw = result;
                        self.product_search_result_pretty = pretty.unwrap_or_default();
                    }
                }
                BackgroundMessage::ProcessSelectedComplete { success, error } => {
                    self.process_all_running = false;
                    self.process_all_progress = None;
                    if success {
                        info!("Processed 1 file successfully.");
                    } else {
                        error!(
                            "Failed to process file: {}",
                            error.unwrap_or_else(|| "Unknown error".to_string())
                        );
                    }
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
fn apply_rules_seq(
    files: &[PathBuf],
    rules: &[RenameRule],
    max_name_length: usize,
) -> Vec<PathBuf> {
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
