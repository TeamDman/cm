use std::path::Path;

/// Open Explorer and select the provided items.
///
/// `cm` only uses this with a single path today, but the function accepts a
/// slice so existing call sites can stay unchanged.
///
/// # Errors
///
/// Returns an error if Explorer cannot be started.
#[cfg(windows)]
pub fn open_folder_and_select_items<P: AsRef<Path>>(paths: &[P]) -> eyre::Result<()> {
    for path in paths {
        open_single_path(path.as_ref())?;
    }

    Ok(())
}

#[cfg(windows)]
fn open_single_path(path: &Path) -> eyre::Result<()> {
    use std::ffi::OsString;
    use std::process::Command;

    let canonical_path = dunce::canonicalize(path)?;
    let mut command = Command::new("explorer.exe");

    if canonical_path.is_dir() {
        command.arg(canonical_path.as_os_str());
    } else {
        let mut select_arg = OsString::from("/select,");
        select_arg.push(canonical_path.as_os_str());
        command.arg(select_arg);
    }

    command.spawn()?;
    Ok(())
}

/// Open Explorer and select the provided items.
///
/// # Errors
///
/// Always returns an error on non-Windows platforms.
#[cfg(not(windows))]
pub fn open_folder_and_select_items<P: AsRef<Path>>(_paths: &[P]) -> eyre::Result<()> {
    eyre::bail!("Explorer selection is only supported on Windows")
}
