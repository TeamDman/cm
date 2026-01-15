use crate::app_home::AppHome;
use glob::glob;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tracing::warn;

/// Returns the path to the `inputs.txt` file in the given `AppHome`
fn inputs_file_path(home: &AppHome) -> PathBuf {
    home.file_path("inputs.txt")
}

/// Load persisted inputs (one per line). Returns canonicalized `PathBufs` as stored.
///
/// # Errors
///
/// Returns an error if reading the inputs file fails.
pub fn load_inputs(home: &AppHome) -> eyre::Result<Vec<PathBuf>> {
    let path = inputs_file_path(home);
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
    let path = inputs_file_path(home);
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
///
/// # Errors
///
/// Returns an error if globbing, canonicalizing paths, or loading inputs fails.
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
///
/// # Errors
///
/// Returns an error if globbing or loading inputs fails.
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

/// Add a list of paths to the persisted inputs. Paths are canonicalized before storing.
///
/// # Errors
///
/// Returns an error if canonicalizing paths, loading inputs, or saving inputs fails.
pub fn add_paths(home: &AppHome, paths: &[PathBuf]) -> eyre::Result<Vec<PathBuf>> {
    let mut new = BTreeSet::new();

    for p in paths {
        // canonicalize the matched path (fail if it cannot be canonicalized)
        let cp = dunce::canonicalize(p)?;
        new.insert(cp);
    }

    if new.is_empty() {
        return Ok(Vec::new());
    }

    let mut current = load_inputs(home)?.into_iter().collect::<BTreeSet<_>>();
    let added: Vec<PathBuf> = new.difference(&current).cloned().collect();

    if added.is_empty() {
        return Ok(Vec::new());
    }

    for p in &added {
        current.insert(p.clone());
    }

    save_inputs(home, &current)?;
    Ok(added)
}

/// Remove all persisted inputs (clear the inputs list)
///
/// # Errors
///
/// Returns an error if file operations fail.
pub fn clear_all(home: &AppHome) -> eyre::Result<()> {
    // Remove the file if it exists; otherwise create an empty file to be explicit
    let path = inputs_file_path(home);
    if path.exists() {
        fs::remove_file(&path)?;
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let _ = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
    }
    Ok(())
}

/// Remove a single path from the persisted inputs. Returns true if the path was present and removed.
///
/// # Errors
///
/// Returns an error if loading or saving inputs fails.
pub fn remove_path(home: &AppHome, path_to_remove: &PathBuf) -> eyre::Result<bool> {
    let mut current = load_inputs(home)?.into_iter().collect::<BTreeSet<_>>();
    let was_present = current.remove(path_to_remove);
    if was_present {
        save_inputs(home, &current)?;
    }
    Ok(was_present)
}

/// Return all files contained in the persisted input paths.
/// If an input path is a file it is included; if it's a directory, all descendant files are included.
///
/// # Errors
///
/// Returns an error if loading inputs or reading directories fails.
pub fn list_files(home: &AppHome) -> eyre::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for p in load_inputs(home)? {
        if p.is_file() {
            files.push(p);
        } else if p.is_dir() {
            add_files_from_dir(&p, &mut files)?;
        }
    }
    Ok(files)
}

fn add_files_from_dir(dir: &PathBuf, out: &mut Vec<PathBuf>) -> eyre::Result<()> {
    for entry in fs::read_dir(dir)? {
        match entry {
            Ok(ent) => {
                let p = ent.path();
                if p.is_file() {
                    out.push(p);
                } else if p.is_dir() {
                    add_files_from_dir(&p, out)?;
                }
            }
            Err(e) => {
                warn!("Failed to read dir entry in {}: {}", dir.display(), e);
            }
        }
    }
    Ok(())
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

    #[test]
    fn add_paths_and_clear_all() -> eyre::Result<()> {
        let td = tempdir()?;
        let home = AppHome(td.path().to_path_buf());

        let file1 = td.path().join("one2.txt");
        File::create(&file1)?;

        // add single path via add_paths
        let added = add_paths(&home, &[file1.clone()])?;
        assert_eq!(added.len(), 1);

        let listed = load_inputs(&home)?;
        assert_eq!(listed.len(), 1);

        // clear all
        clear_all(&home)?;
        let listed2 = load_inputs(&home)?;
        assert_eq!(listed2.len(), 0);

        Ok(())
    }

    #[test]
    fn list_files_recurses() -> eyre::Result<()> {
        let td = tempdir()?;
        let home = AppHome(td.path().to_path_buf());

        // create a dir with nested files
        let dir = td.path().join("d1");
        fs::create_dir_all(&dir)?;
        let f1 = dir.join("a.txt");
        let f2 = dir.join("sub").join("b.png");
        fs::create_dir_all(f2.parent().unwrap())?;
        File::create(&f1)?;
        File::create(&f2)?;

        // add the directory as an input
        let _ = add_paths(&home, &[dir.clone()])?;

        let files = list_files(&home)?;
        // Should contain both a.txt and b.png
        assert!(files.iter().any(|p| p.file_name().unwrap() == "a.txt"));
        assert!(files.iter().any(|p| p.file_name().unwrap() == "b.png"));

        Ok(())
    }
}
