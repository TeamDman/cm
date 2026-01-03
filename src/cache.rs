//! API response caching layer.
//!
//! Caches raw API responses to:
//! - Avoid hammering the API during development/debugging
//! - Allow diagnosis of deserialization issues by examining stored plaintext

use directories_next::ProjectDirs;
use once_cell::sync::Lazy;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;
use tracing::warn;

/// The cache home directory for API responses.
pub static CACHE_HOME: Lazy<CacheHome> = Lazy::new(|| match CacheHome::resolve() {
    Ok(c) => c,
    Err(e) => {
        warn!("Failed to resolve cache home: {}", e);
        CacheHome(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
});

/// Helper that resolves the application cache directory.
#[derive(Clone, Debug)]
pub struct CacheHome(pub PathBuf);

impl CacheHome {
    /// Resolve the CacheHome according to:
    /// * If `CM_CACHE_DIR` env var is set, use that directory
    /// * Otherwise use the platform `ProjectDirs::cache_dir()` for teamdman/cm
    pub fn resolve() -> eyre::Result<CacheHome> {
        if let Ok(override_dir) = std::env::var("CM_CACHE_DIR") {
            return Ok(CacheHome(PathBuf::from(override_dir)));
        }
        if let Some(pd) = ProjectDirs::from("", "teamdman", "cm") {
            Ok(CacheHome(pd.cache_dir().to_path_buf()))
        } else {
            Err(eyre::eyre!("Could not determine cache directory"))
        }
    }

    /// Returns the path to the api_responses subdirectory.
    pub fn api_responses_dir(&self) -> PathBuf {
        self.0.join("api_responses")
    }
}

impl std::ops::Deref for CacheHome {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.as_path()
    }
}

/// A cache entry for an API response.
#[derive(Debug)]
pub struct CacheEntry {
    /// Directory for this cache entry.
    pub dir: PathBuf,
}

impl CacheEntry {
    /// Create a new cache entry for the given URL.
    pub fn for_url(url: &str) -> Self {
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(url.as_bytes());
            let result = hasher.finalize();
            hex::encode(result)
        };
        // Use first 16 chars of hash for shorter paths
        let short_hash = &hash[..16];
        let dir = CACHE_HOME.api_responses_dir().join(short_hash);
        Self { dir }
    }

    /// Path to the response body file.
    pub fn response_path(&self) -> PathBuf {
        self.dir.join("response.txt")
    }

    /// Path to the URL file.
    pub fn url_path(&self) -> PathBuf {
        self.dir.join("url.txt")
    }

    /// Path to the timestamps file.
    pub fn timestamps_path(&self) -> PathBuf {
        self.dir.join("timestamps.txt")
    }

    /// Check if a cached response exists.
    pub fn exists(&self) -> bool {
        self.response_path().exists()
    }

    /// Read the cached response body if it exists.
    pub fn read(&self) -> eyre::Result<Option<String>> {
        if !self.exists() {
            return Ok(None);
        }

        let body = std::fs::read_to_string(self.response_path())?;

        // Append access timestamp
        self.append_timestamp()?;

        debug!(cache_dir = %self.dir.display(), "Cache hit");
        Ok(Some(body))
    }

    /// Write a response to the cache.
    pub fn write(&self, url: &str, body: &str) -> eyre::Result<()> {
        std::fs::create_dir_all(&self.dir)?;

        std::fs::write(self.response_path(), body)?;
        std::fs::write(self.url_path(), url)?;
        self.append_timestamp()?;

        debug!(cache_dir = %self.dir.display(), "Cached response");
        Ok(())
    }

    /// Append current timestamp to the timestamps file.
    fn append_timestamp(&self) -> eyre::Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.timestamps_path())?;
        writeln!(file, "{}", timestamp)?;
        Ok(())
    }
}

/// Clean the entire API response cache directory.
pub fn clean_cache() -> eyre::Result<CleanResult> {
    let cache_dir = CACHE_HOME.api_responses_dir();
    let mut result = CleanResult::default();

    if !cache_dir.exists() {
        return Ok(result);
    }

    for entry in std::fs::read_dir(&cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
            result.entries_removed += 1;
        }
    }

    // Remove the api_responses directory itself if empty
    if std::fs::read_dir(&cache_dir)?.next().is_none() {
        std::fs::remove_dir(&cache_dir)?;
    }

    Ok(result)
}

/// Result of a cache clean operation.
#[derive(Debug, Default)]
pub struct CleanResult {
    /// Number of cache entries removed.
    pub entries_removed: usize,
}
