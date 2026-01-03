use crate::app_home::AppHome;
use dunce;
use glob::glob;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tracing::warn;

/// Returns the path to the `inputs.txt` file in the given `AppHome`
fn inputs_file_path(home: &AppHome) -> eyre::Result<PathBuf> {
    Ok(home.file_path("inputs.txt"))
}

/// Load persisted inputs (one per line). Returns canonicalized PathBufs as stored.
pub fn load_inputs(home: &AppHome) -> eyre::Result<Vec<PathBuf>> {
    let path = inputs_file_path(home)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let s = fs::read_to_string(&path)?;
    let mut v = Vec::new();
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        v.push(PathBuf::from(trimmed));
    }
    Ok(v)
}

/// Persist the provided set of canonical paths to the inputs file (one per line)
fn save_inputs(home: &AppHome, paths: &BTreeSet<PathBuf>) -> eyre::Result<()> {
    let path = inputs_file_path(home)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;
    for p in paths {
        writeln!(f, "{}", p.display())?;
    }
    Ok(())
}

/// Add paths resolved from a glob pattern. Each matched path is canonicalized before being stored.
/// Returns the list of newly added canonical paths.
pub fn add_from_glob(home: &AppHome, pattern: &str) -> eyre::Result<Vec<PathBuf>> {
    let mut new = BTreeSet::new();

    for entry in glob(pattern)? {
        let p = entry.map_err(|e| eyre::eyre!("Glob pattern error: {}", e))?;
        // canonicalize the matched path (fail if it cannot be canonicalized)
        let cp = dunce::canonicalize(&p)?;
        new.insert(cp);
    }

    if new.is_empty() {
        // No matches is not an error; return empty vector
        return Ok(Vec::new());
    }

    let mut current = load_inputs(home)?.into_iter().collect::<BTreeSet<_>>();
    let added: Vec<PathBuf> = new.difference(&current).cloned().collect();

    if added.is_empty() {
        // Nothing to change
        return Ok(Vec::new());
    }

    for p in &added {
        current.insert(p.clone());
    }

    save_inputs(home, &current)?;
    Ok(added)
}

/// Remove paths that match the glob pattern. Returns removed canonical paths.
pub fn remove_from_glob(home: &AppHome, pattern: &str) -> eyre::Result<Vec<PathBuf>> {
    let mut to_remove = BTreeSet::new();

    for entry in glob(pattern)? {
        let p = entry.map_err(|e| eyre::eyre!("Glob pattern error: {}", e))?;
        let cp = match dunce::canonicalize(&p) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to canonicalize {}: {}", p.display(), e);
                continue;
            }
        };
        to_remove.insert(cp);
    }

    if to_remove.is_empty() {
        return Ok(Vec::new());
    }

    let mut current = load_inputs(home)?.into_iter().collect::<BTreeSet<_>>();
    let removed: Vec<PathBuf> = current.intersection(&to_remove).cloned().collect();

    if removed.is_empty() {
        return Ok(Vec::new());
    }

    for p in &removed {
        current.remove(p);
    }

    save_inputs(home, &current)?;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn add_list_remove_cycle() -> eyre::Result<()> {
        let td = tempdir()?;
        // Create a temporary AppHome pointing at the tempdir so we don't modify global env
        let home = AppHome(td.path().to_path_buf());

        let file1 = td.path().join("one.txt");
        let file2 = td.path().join("two.txt");
        File::create(&file1)?;
        File::create(&file2)?;

        // add both
        let added = add_from_glob(&home, &format!("{}/*.txt", td.path().display()))?;
        assert_eq!(added.len(), 2);

        // list
        let listed = load_inputs(&home)?;
        assert_eq!(listed.len(), 2);

        // remove one
        let removed = remove_from_glob(&home, file1.to_str().unwrap())?;
        assert_eq!(removed.len(), 1);

        let remaining = load_inputs(&home)?;
        assert_eq!(remaining.len(), 1);
        assert!(remaining[0] != removed[0]);

        Ok(())
    }
}
