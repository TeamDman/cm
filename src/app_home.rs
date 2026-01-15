use directories_next::ProjectDirs;
use std::env;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

/// Helper that resolves the application config directory and provides helper methods
#[derive(Clone, Debug, PartialEq)]
pub struct AppHome(pub PathBuf);

impl AppHome {
    /// Returns a `PathBuf` for a filename under the app config dir
    #[must_use] 
    pub fn file_path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Create directories for the app home if needed
    pub fn ensure_dir(&self) -> eyre::Result<()> {
        std::fs::create_dir_all(&self.0)?;
        Ok(())
    }

    /// Resolve the `AppHome` according to the same rules used previously:
    /// * If `CM_CONFIG_DIR` env var is set, use that directory
    /// * Otherwise use the platform `ProjectDirs::config_dir()` for teamdman/cm
    pub fn resolve() -> eyre::Result<AppHome> {
        if let Ok(override_dir) = env::var("CM_CONFIG_DIR") {
            return Ok(AppHome(PathBuf::from(override_dir)));
        }
        if let Some(pd) = ProjectDirs::from("", "teamdman", "cm") {
            Ok(AppHome(pd.config_dir().to_path_buf()))
        } else {
            Err(eyre::eyre!("Could not determine config directory"))
        }
    }

    /// Returns true if this `AppHome` equals the global `APP_HOME`
    #[must_use] 
    pub fn is_default(&self) -> bool {
        // Compare absolute paths
        self.0 == APP_HOME.0
    }
}

impl Deref for AppHome {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.as_path()
    }
}

use tracing::warn;

/// Cached `AppHome` instance
pub static APP_HOME: LazyLock<AppHome> = LazyLock::new(|| match AppHome::resolve() {
    Ok(a) => a,
    Err(e) => {
        warn!("Warning: failed to resolve app home: {}", e);
        // Fallback to current directory to avoid panic; behavior mirrors previous code warning behavior
        AppHome(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
});
