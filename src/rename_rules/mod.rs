mod rename_rule;

use crate::app_home::AppHome;
pub use rename_rule::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::Mutex;
use uuid::Uuid;

const DIR_NAME: &str = "rename-rules";
const FILE_EXT: &str = "txt";

// Cache for global rules when accessed via APP_HOME
static GLOBAL_RULE_CACHE: LazyLock<Mutex<Option<Vec<RenameRule>>>> =
    LazyLock::new(|| Mutex::new(None));
/// Ensure the rename rules directory exists and return its path
fn dir_for(home: &AppHome) -> eyre::Result<PathBuf> {
    let dir = home.file_path(DIR_NAME);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Public helper to get the path to the rename rules directory
///
/// # Errors
///
/// Returns an error if creating the rules directory fails.
pub fn rules_dir(home: &AppHome) -> eyre::Result<PathBuf> {
    dir_for(home)
}

/// List rule file paths sorted by name
fn list_rule_files(home: &AppHome) -> eyre::Result<Vec<PathBuf>> {
    let dir = dir_for(home)?;
    let mut v: Vec<_> = fs::read_dir(dir)?
        .filter_map(std::result::Result::ok)
        .map(|d| d.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some(FILE_EXT))
        .collect();
    v.sort();
    Ok(v)
}

/// Add a new rule file and return its assigned UUID
/// # Errors
/// Returns an error if the rule file cannot be written.
/// # Panics
/// Panics if the global rule cache mutex cannot be locked.
pub fn add_rule(home: &AppHome, rule: &RenameRule) -> eyre::Result<Uuid> {
    let dir = dir_for(home)?;
    let filename = format!("{}.{}", rule.id, FILE_EXT);
    let path = dir.join(filename);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;
    let content = rule.to_file_text();
    writeln!(f, "{content}")?;
    // invalidate cache if default app home
    if home.is_default() {
        let mut cache = GLOBAL_RULE_CACHE.lock().unwrap();
        *cache = None;
    }
    Ok(rule.id)
}

/// Remove a rule by UUID
/// # Errors
/// Returns an error if the rule file cannot be removed.
/// # Panics
/// Panics if the global rule cache mutex cannot be locked.
pub fn remove_rule(home: &AppHome, id: Uuid) -> eyre::Result<bool> {
    let dir = dir_for(home)?;
    let path = dir.join(format!("{id}.{FILE_EXT}"));
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path)?;
    // invalidate cache if default app home
    if home.is_default() {
        let mut cache = GLOBAL_RULE_CACHE.lock().unwrap();
        *cache = None;
    }
    Ok(true)
}

/// Write rule to file by id (create or overwrite)
/// # Errors
/// Returns an error if the rule file cannot be written.
/// # Panics
/// Panics if the global rule cache mutex cannot be locked.
pub fn write_rule(home: &AppHome, rule: &RenameRule) -> eyre::Result<()> {
    let dir = dir_for(home)?;
    let path = dir.join(format!("{}.{}", rule.id, FILE_EXT));
    let mut f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;
    let content = rule.to_file_text();
    write!(f, "{content}")?;

    // invalidate cache if default app home
    if home.is_default() {
        let mut cache = GLOBAL_RULE_CACHE.lock().unwrap();
        *cache = None;
    }
    Ok(())
}

/// List parsed rules with their indices
/// # Errors
/// Returns an error if the rules directory cannot be read or rules cannot be parsed.
/// # Panics
/// Panics if the global rule cache mutex cannot be locked.
pub fn list_rules(home: &AppHome) -> eyre::Result<Vec<(usize, RenameRule)>> {
    // If this is the default app home, use cached list when available
    if home.is_default() {
        let cache_lock = GLOBAL_RULE_CACHE.lock().unwrap();
        if let Some(cached) = cache_lock.as_ref() {
            let out = cached
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, r)| (i + 1, r))
                .collect();
            return Ok(out);
        }
    }

    let files = list_rule_files(home)?;
    let mut out_rules = Vec::new();
    for p in &files {
        if let Ok(text) = std::fs::read_to_string(p)
            && let Ok(mut rule) = RenameRule::from_file_text(&text)
        {
            // Parse id from filename (stem)
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str())
                && let Ok(id) = Uuid::parse_str(stem)
            {
                rule.id = id;
            }
            out_rules.push(rule);
        }
    }

    // If default home, populate cache
    if home.is_default() {
        let mut cache_lock = GLOBAL_RULE_CACHE.lock().unwrap();
        *cache_lock = Some(out_rules.clone());
    }

    let out = out_rules
        .into_iter()
        .enumerate()
        .map(|(i, r)| (i + 1, r))
        .collect();
    Ok(out)
}
