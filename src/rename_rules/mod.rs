mod rename_rule;
mod modifier;
mod when_expr;

pub use rename_rule::*;
pub use modifier::*;
pub use when_expr::*;

use crate::app_home::AppHome;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const DIR_NAME: &str = "rename-rules";
const FILE_EXT: &str = "txt";

/// Ensure the rename rules directory exists and return its path
fn dir_for(home: &AppHome) -> eyre::Result<PathBuf> {
    let dir = home.file_path(DIR_NAME);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// List rule file paths sorted by name
fn list_rule_files(home: &AppHome) -> eyre::Result<Vec<PathBuf>> {
    let dir = dir_for(home)?;
    let mut v: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|d| d.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some(FILE_EXT))
        .collect();
    v.sort();
    Ok(v)
}

/// Add a new rule file and return its assigned 1-based index
pub fn add_rule(home: &AppHome, rule: &RenameRule) -> eyre::Result<usize> {
    let dir = dir_for(home)?;
    let files = list_rule_files(home)?;
    // next index is files.len() + 1
    let idx = files.len() + 1;
    let filename = format!("{:03}.{}", idx, FILE_EXT);
    let path = dir.join(filename);
    let mut f = fs::OpenOptions::new().create(true).write(true).truncate(true).open(&path)?;
    let content = rule.to_file_text();
    writeln!(f, "{}", content)?;
    Ok(idx)
}

/// Remove a rule by 1-based index and renumber remaining files
pub fn remove_rule(home: &AppHome, idx: usize) -> eyre::Result<bool> {
    let mut files = list_rule_files(home)?;
    if idx == 0 || idx > files.len() {
        return Ok(false);
    }
    // remove file
    let removed = files.remove(idx - 1);
    fs::remove_file(&removed)?;
    // renumber remaining
    let dir = dir_for(home)?;
    for (i, p) in files.iter().enumerate() {
        let new_name = format!("{:03}.{}", i + 1, FILE_EXT);
        let new_path = dir.join(new_name);
        if p != &new_path {
            fs::rename(p, new_path)?;
        }
    }
    Ok(true)
}

/// List parsed rules with their indices
pub fn list_rules(home: &AppHome) -> eyre::Result<Vec<(usize, RenameRule)>> {
    let files = list_rule_files(home)?;
    let mut out = Vec::new();
    for (i, p) in files.iter().enumerate() {
        if let Ok(text) = std::fs::read_to_string(p) {
            if let Ok(rule) = RenameRule::from_file_text(&text) {
                out.push((i + 1, rule));
            }
        }
    }
    Ok(out)
}