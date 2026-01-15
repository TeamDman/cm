use crate::app_home::APP_HOME;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tracing::warn;

/// A strongly-typed wrapper around the max name length
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaxNameLength(pub usize);

impl MaxNameLength {
    /// Default max name length
    pub const DEFAULT: usize = 50;
    const FILE_NAME: &'static str = "max_name_length.txt";

    /// Loads resolving rules:
    /// 1. If $`CM_MAX_NAME_LENGTH` is set -> use it (and DO NOT create file)
    /// 2. Otherwise, look for `${config_dir}/max_name_length.txt`
    ///    - if file exists, parse its trimmed contents
    ///    - otherwise, create the file containing the default and return default
    ///
    /// # Errors
    ///
    /// Returns an error if the config file path cannot be determined or reading the file fails.
    pub fn load() -> eyre::Result<MaxNameLength> {
        // 1. Env var
        if let Ok(envv) = env::var("CM_MAX_NAME_LENGTH") {
            if let Ok(v) = envv.trim().parse::<usize>() {
                return Ok(MaxNameLength(v));
            }
            warn!(
                "Invalid CM_MAX_NAME_LENGTH '{}', falling back to file/default",
                envv
            );
        }

        // 2. Config file under config_dir/teamdman/cm/max_name_length.txt
        let path = Self::config_file_path()?;
        if path.exists() {
            let s = fs::read_to_string(&path)?.trim().to_string();
            if let Ok(v) = s.parse::<usize>() {
                return Ok(MaxNameLength(v));
            }
            warn!(
                "Invalid {} contents: '{}', resetting to default",
                path.display(),
                s
            );
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
        f.write_all(Self::DEFAULT.to_string().as_bytes())?;
        f.flush()?;
        Ok(MaxNameLength(Self::DEFAULT))
    }

    /// Returns the path the file should live at
    ///
    /// # Errors
    ///
    /// This function does not return any errors.
    pub fn config_file_path() -> eyre::Result<PathBuf> {
        Ok(APP_HOME.file_path(Self::FILE_NAME))
    }

    /// Set the value by writing to the config file (creates dirs if needed).
    /// This also updates the in-memory static so subsequent calls in the same
    /// process immediately observe the new value.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file path cannot be determined or writing to the file fails.
    pub fn set_to(value: usize) -> eyre::Result<()> {
        let path = Self::config_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, value.to_string().as_bytes())?;

        // Update in-memory cached value so callers see the change immediately
        crate::MAX_NAME_LENGTH.store(value, Ordering::SeqCst);
        Ok(())
    }

    /// Convenience accessor
    #[must_use]
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

/// Public static that initializes using the rules described above.
/// Backed by `AtomicUsize` so the `set_to` method can update it at runtime.
pub static MAX_NAME_LENGTH: LazyLock<AtomicUsize> = LazyLock::new(|| {
    let initial = MaxNameLength::load()
        .map(|m| m.as_usize())
        .unwrap_or(MaxNameLength::DEFAULT);
    AtomicUsize::new(initial)
});
