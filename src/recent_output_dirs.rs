use crate::app_home::AppHome;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

const FILE_NAME: &str = "recent_output_dirs.txt";
const MAX_RECENT_OUTPUT_DIRS: usize = 10;

fn file_path(home: &AppHome) -> PathBuf {
    home.file_path(FILE_NAME)
}

/// Load recent output directory picks, most recent first.
///
/// # Errors
/// Returns an error if the recent output directory file exists but cannot be read.
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

/// Remember an output directory pick, moving it to the front of the list.
///
/// # Errors
/// Returns an error if the recent output directory file cannot be written.
pub fn remember(home: &AppHome, output_dir: &Path) -> eyre::Result<Vec<PathBuf>> {
    let mut recent = load(home)?;
    recent.retain(|path| path != output_dir);
    recent.insert(0, output_dir.to_path_buf());
    recent.truncate(MAX_RECENT_OUTPUT_DIRS);
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
    for output_dir in recent {
        writeln!(file, "{}", output_dir.display())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn remember_keeps_most_recent_first_and_dedupes() -> eyre::Result<()> {
        let dir = tempdir()?;
        let home = AppHome(dir.path().to_path_buf());
        let first = PathBuf::from("C:/out/one");
        let second = PathBuf::from("C:/out/two");

        remember(&home, &first)?;
        remember(&home, &second)?;
        remember(&home, &first)?;

        let recent = load(&home)?;
        assert_eq!(recent, vec![first, second]);
        Ok(())
    }
}
