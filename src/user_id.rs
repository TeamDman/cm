use crate::app_home::APP_HOME;
use chrono::DateTime;
use chrono::Duration;
use chrono::Local;
use chrono::Utc;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserId(pub Uuid);

impl UserId {
    const FILE_NAME: &'static str = "user_id.txt";
    const TTL_DAYS: i64 = 4;

    /// # Errors
    ///
    /// This function does not return any errors.
    pub fn config_file_path() -> eyre::Result<PathBuf> {
        Ok(APP_HOME.file_path(Self::FILE_NAME))
    }

    /// # Errors
    ///
    /// Returns an error if loading or parsing the user ID fails.
    pub fn load() -> eyre::Result<UserId> {
        let path = Self::config_file_path()?;
        if path.exists() {
            let s = fs::read_to_string(&path)?;
            let mut lines = s.lines();
            if let Some(id_line) = lines.next()
                && let Some(exp_line) = lines.next()
            {
                let id = Uuid::parse_str(id_line.trim())?;
                // New format: `expires {timestamp}` -> extract timestamp by splitting from the right
                let exp_str = exp_line.rsplit(' ').next().unwrap_or(exp_line.trim());
                let exp_dt = DateTime::parse_from_rfc3339(exp_str).map_err(|e| eyre::eyre!(e))?;

                if exp_dt.with_timezone(&Utc) > Utc::now() {
                    return Ok(UserId(id));
                }
            }
        }

        // missing or expired -> generate and persist
        let id = Uuid::new_v4();
        let expiry = Local::now() + Duration::days(Self::TTL_DAYS);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        f.write_all(format!("{}\nexpires {}", id, expiry.to_rfc3339()).as_bytes())?;
        f.flush()?;
        Ok(UserId(id))
    }

    #[must_use]
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

use tracing::warn;

pub static USER_ID: LazyLock<UserId> = LazyLock::new(|| match UserId::load() {
    Ok(u) => u,
    Err(e) => {
        warn!(
            "Warning: failed to load user id: {}. Generating new one.",
            e
        );
        UserId(Uuid::new_v4())
    }
});
