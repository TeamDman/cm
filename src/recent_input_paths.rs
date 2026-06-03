use crate::app_home::AppHome;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

const FILE_NAME: &str = "recent_input_paths.txt";
const MAX_RECENT_INPUT_PATHS: usize = 10;

fn file_path(home: &AppHome) -> PathBuf {
    home.file_path(FILE_NAME)
}

/// Load recent input path picks, most recent first.
///
/// # Errors
/// Returns an error if the recent input path file exists but cannot be read.
pub fn load(home: &AppHome) -> eyre::Result<Vec<PathBuf>> {
    let path = file_path(home);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(path)?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// Remember an input path pick, moving it to the front of the list.
///
/// # Errors
/// Returns an error if the input path cannot be canonicalized or the recent file
/// cannot be written.
pub fn remember(home: &AppHome, input_path: &Path) -> eyre::Result<Vec<PathBuf>> {
    remember_many(home, &[input_path.to_path_buf()])
}

/// Remember multiple input path picks, preserving their selected order at the front.
///
/// # Errors
/// Returns an error if any input path cannot be canonicalized or the recent file
/// cannot be written.
pub fn remember_many(home: &AppHome, input_paths: &[PathBuf]) -> eyre::Result<Vec<PathBuf>> {
    let mut recent = load(home)?;
    let selected = input_paths
        .iter()
        .map(dunce::canonicalize)
        .collect::<Result<Vec<_>, _>>()?;

    for input_path in selected.iter().rev() {
        recent.retain(|path| path != input_path);
        recent.insert(0, input_path.clone());
    }
    recent.truncate(MAX_RECENT_INPUT_PATHS);
    save(home, &recent)?;
    Ok(recent)
}

fn save(home: &AppHome, recent: &[PathBuf]) -> eyre::Result<()> {
    let path = file_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    for input_path in recent {
        writeln!(file, "{}", input_path.display())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn remember_many_keeps_recent_first_dedupes_and_limits() -> eyre::Result<()> {
        let dir = tempdir()?;
        let home = AppHome(dir.path().join("home"));
        let mut picks = Vec::new();

        for idx in 0..12 {
            let path = dir.path().join(format!("input-{idx}"));
            fs::create_dir_all(&path)?;
            picks.push(path);
        }

        remember_many(&home, &picks)?;
        remember(&home, &picks[2])?;

        let recent = load(&home)?;
        assert_eq!(recent.len(), MAX_RECENT_INPUT_PATHS);
        assert_eq!(recent[0], dunce::canonicalize(&picks[2])?);
        assert_eq!(recent.iter().filter(|path| **path == recent[0]).count(), 1);
        Ok(())
    }
}
