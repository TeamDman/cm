//! Shared application state for the CM GUI

use crate::app_home::APP_HOME;
use crate::inputs;
use crate::rename_rules::RenameRule;
use crate::MAX_NAME_LENGTH;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tracing::info;

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
    /// Currently previewed input image path
    pub input_preview_path: Option<PathBuf>,
    /// Currently previewed output image path
    pub output_preview_path: Option<PathBuf>,
    /// Whether we've initialized
    pub initialized: bool,
}

impl Default for AppState {
    fn default() -> Self {
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
            input_preview_path: None,
            output_preview_path: None,
            initialized: false,
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
