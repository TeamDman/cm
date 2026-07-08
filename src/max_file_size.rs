use crate::app_home::AppHome;
use std::fs;
use std::path::PathBuf;
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaxFileSize(pub Option<u64>);

impl MaxFileSize {
    const FILE_NAME: &'static str = "max_file_size_bytes.txt";
    const DISABLED: &'static str = "off";

    pub fn load(app_home: &AppHome) -> eyre::Result<MaxFileSize> {
        let path = Self::config_file_path(app_home);
        if !path.exists() {
            return Ok(MaxFileSize(None));
        }

        let value = fs::read_to_string(&path)?.trim().to_string();
        if value.eq_ignore_ascii_case(Self::DISABLED) || value.is_empty() {
            return Ok(MaxFileSize(None));
        }

        match value.parse::<u64>() {
            Ok(bytes) => Ok(MaxFileSize(Some(bytes))),
            Err(_) => {
                warn!(
                    "Invalid {} contents: '{}', disabling max file size",
                    path.display(),
                    value
                );
                Ok(MaxFileSize(None))
            }
        }
    }

    #[must_use]
    pub fn config_file_path(app_home: &AppHome) -> PathBuf {
        app_home.file_path(Self::FILE_NAME)
    }

    pub fn set_to(app_home: &AppHome, value: Option<u64>) -> eyre::Result<()> {
        let path = Self::config_file_path(app_home);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = value.map_or_else(|| Self::DISABLED.to_string(), |bytes| bytes.to_string());
        fs::write(&path, contents.as_bytes())?;
        Ok(())
    }

    #[must_use]
    pub fn as_option(&self) -> Option<u64> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_setting_loads_disabled() -> eyre::Result<()> {
        let dir = tempdir()?;
        let app_home = AppHome(dir.path().to_path_buf());

        assert_eq!(MaxFileSize::load(&app_home)?.as_option(), None);
        Ok(())
    }

    #[test]
    fn set_and_load_enabled_value() -> eyre::Result<()> {
        let dir = tempdir()?;
        let app_home = AppHome(dir.path().to_path_buf());

        MaxFileSize::set_to(&app_home, Some(1024 * 1024))?;

        assert_eq!(MaxFileSize::load(&app_home)?.as_option(), Some(1024 * 1024));
        Ok(())
    }

    #[test]
    fn set_and_load_disabled_value() -> eyre::Result<()> {
        let dir = tempdir()?;
        let app_home = AppHome(dir.path().to_path_buf());

        MaxFileSize::set_to(&app_home, None)?;

        assert_eq!(MaxFileSize::load(&app_home)?.as_option(), None);
        Ok(())
    }
}
