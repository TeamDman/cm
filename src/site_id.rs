use crate::app_home::APP_HOME;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use tracing::warn;

/// A strongly-typed wrapper around the site id string
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteId(pub String);

impl SiteId {
    /// Default site id from README
    pub const DEFAULT: &'static str = "4y9u7l";

    /// Loads resolving rules:
    /// 1. If $`CM_SITE_ID` is set -> use it (and DO NOT create file)
    /// 2. Otherwise, look for `${config_dir}/cm_site_id.txt`
    ///    - if file exists, use its trimmed contents
    ///    - otherwise, create the file containing the default and return default
    pub fn load() -> eyre::Result<SiteId> {
        // 1. Env var
        if let Ok(envv) = env::var("CM_SITE_ID") {
            return Ok(SiteId(envv));
        }

        // 2. Config file under config_dir/teamdman/cm/cm_site_id.txt
        let path = Self::config_file_path()?;
        if path.exists() {
            let s = fs::read_to_string(&path)?.trim().to_string();
            return Ok(SiteId(s));
        }

        // create containing default
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        f.write_all(Self::DEFAULT.as_bytes())?;
        f.flush()?;
        Ok(SiteId(Self::DEFAULT.to_string()))
    }

    /// Returns the path the file should live at
    pub fn config_file_path() -> eyre::Result<PathBuf> {
        Ok(APP_HOME.file_path("cm_site_id.txt"))
    }

    /// Set the site id by writing to the config file (creates dirs if needed).
    /// Note: this does not update any existing in-memory static cache; the new value
    /// will be picked up on next run or by calling `SiteId::load()` explicitly.
    pub fn set_to(id: &str) -> eyre::Result<()> {
        let path = Self::config_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, id.as_bytes())?;
        Ok(())
    }

    /// Convenience accessor
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Public static site id that initializes using the rules described above.
pub static SITE_ID: LazyLock<SiteId> = LazyLock::new(|| {
    match SiteId::load() {
        Ok(s) => s,
        Err(e) => {
            // If load fails, fallback to DEFAULT and log a warning
            warn!(
                "Warning: failed to load site id: {}. Using default {}",
                e,
                SiteId::DEFAULT
            );
            SiteId(SiteId::DEFAULT.to_string())
        }
    }
});
