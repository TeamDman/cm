use eyre::{Context, Result, bail};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DEFAULT_REACTOR_SHELL_EXE: &str = r"D:\Repos\rust\windows-rs\target\debug\cm-shell.exe";
const DEFAULT_REACTOR_SHELL_MANIFEST: &str =
    r"D:\Repos\rust\windows-rs\crates\samples\reactor\cm-shell\Cargo.toml";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReactorShellMode {
    MainMenu,
    StudioV2,
}

impl ReactorShellMode {
    fn as_arg(self) -> Option<&'static str> {
        match self {
            Self::MainMenu => None,
            Self::StudioV2 => Some("studio-v2"),
        }
    }
}

pub(crate) fn run_blocking(mode: ReactorShellMode) -> Result<()> {
    let mut command = reactor_shell_command(mode)?;
    let status = command
        .status()
        .wrap_err("failed to launch the Reactor shell")?;

    if status.success() {
        Ok(())
    } else {
        bail!("Reactor shell exited with status {status}")
    }
}

pub(crate) fn spawn_detached(mode: ReactorShellMode) -> Result<()> {
    let mut command = reactor_shell_command(mode)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
        .spawn()
        .wrap_err("failed to launch the Reactor shell")?;
    Ok(())
}

fn reactor_shell_command(mode: ReactorShellMode) -> Result<Command> {
    if let Some(exe_path) = configured_exe_path() {
        let mut command = Command::new(exe_path);
        push_mode_arg(&mut command, mode);
        return Ok(command);
    }

    let manifest_path = env::var_os("CM_REACTOR_SHELL_MANIFEST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REACTOR_SHELL_MANIFEST));
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--");
    push_mode_arg(&mut command, mode);
    Ok(command)
}

fn configured_exe_path() -> Option<PathBuf> {
    env::var_os("CM_REACTOR_SHELL_EXE")
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| sibling_exe_path().filter(|path| path.exists()))
        .or_else(|| existing_path(DEFAULT_REACTOR_SHELL_EXE))
}

fn sibling_exe_path() -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .map(|parent| parent.join("cm-shell.exe"))
}

fn existing_path(path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    path.exists().then(|| path.to_path_buf())
}

fn push_mode_arg(command: &mut Command, mode: ReactorShellMode) {
    if let Some(mode) = mode.as_arg() {
        command.arg("--mode").arg(mode);
    }
}
