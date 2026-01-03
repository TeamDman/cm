use crate::app_home::APP_HOME;
use chrono::DateTime;
use chrono::Duration;
use chrono::Local;
use chrono::Utc;
use once_cell::sync::Lazy;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionId(pub Uuid);

impl SessionId {
    const FILE_NAME: &'static str = "session_id.txt";
    const TTL_HOURS: i64 = 1;

    pub fn config_file_path() -> eyre::Result<PathBuf> {
        Ok(APP_HOME.file_path(Self::FILE_NAME))
    }

    pub fn load() -> eyre::Result<SessionId> {
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
                    return Ok(SessionId(id));
                }
            }
        }

        // missing or expired -> generate and persist
        let id = Uuid::new_v4();
        let expiry = Local::now() + Duration::hours(Self::TTL_HOURS);
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
        Ok(SessionId(id))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

use tracing::warn;

pub static SESSION_ID: Lazy<SessionId> = Lazy::new(|| match SessionId::load() {
    Ok(s) => s,
    Err(e) => {
        warn!(
            "Warning: failed to load session id: {}. Generating new one.",
            e
        );
        SessionId(Uuid::new_v4())
    }
});
