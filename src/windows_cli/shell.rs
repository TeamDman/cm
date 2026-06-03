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

/// Launch a new `cm gui` process using the current executable.
///
/// # Errors
///
/// Returns an error if the current executable cannot be resolved or the child
/// process cannot be started.
#[cfg(windows)]
pub fn launch_gui_mode(mode: &str) -> eyre::Result<u32> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = std::env::current_exe()?;
    let child = Command::new(exe)
        .arg("gui")
        .arg("--mode")
        .arg(mode)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    Ok(child.id())
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

/// Launch a new `cm gui` process using the current executable.
///
/// # Errors
///
/// Always returns an error on non-Windows platforms.
#[cfg(not(windows))]
pub fn launch_gui_mode(_mode: &str) -> eyre::Result<u32> {
    eyre::bail!("GUI process launching is only supported on Windows")
}
