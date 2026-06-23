use eyre::Context;
use tracing::warn;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::CreateFileW;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::Storage::FileSystem::FILE_GENERIC_READ;
use windows::Win32::Storage::FileSystem::FILE_GENERIC_WRITE;
use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;
use windows::Win32::Storage::FileSystem::FILE_SHARE_WRITE;
use windows::Win32::Storage::FileSystem::OPEN_EXISTING;
use windows::Win32::System::Console::AttachConsole;
use windows::Win32::System::Console::CONSOLE_MODE;
use windows::Win32::System::Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING;
use windows::Win32::System::Console::FreeConsole;
use windows::Win32::System::Console::GetConsoleMode;
use windows::Win32::System::Console::GetStdHandle;
use windows::Win32::System::Console::STD_ERROR_HANDLE;
use windows::Win32::System::Console::STD_INPUT_HANDLE;
use windows::Win32::System::Console::STD_OUTPUT_HANDLE;
use windows::Win32::System::Console::SetConsoleMode;
use windows::Win32::System::Console::SetStdHandle;
use windows::core::w;

/// Returns the current STDOUT handle, erroring if it is invalid.
pub fn get_console_output_handle() -> eyre::Result<HANDLE> {
    unsafe {
        let handle =
            GetStdHandle(STD_OUTPUT_HANDLE).wrap_err("Failed to get standard output handle")?;
        if handle.is_invalid() {
            Err(windows::core::Error::from_thread()).wrap_err("STD_OUTPUT_HANDLE is invalid")
        } else {
            Ok(handle)
        }
    }
}

/// Attach to the console owned by `pid` and rebind Rust stdio to it.
pub fn console_attach(pid: u32) -> eyre::Result<()> {
    let _ = console_detach();
    unsafe { AttachConsole(pid) }
        .wrap_err_with(|| format!("Failed to attach to console with PID {pid}."))?;
    rebind_std_handles_to_console()?;
    if let Err(err) = enable_ansi_support() {
        warn!(?err, "failed to enable ANSI console support");
    }
    Ok(())
}

fn console_detach() -> eyre::Result<()> {
    unbind_and_close_std_handles_for_detach();
    unsafe { FreeConsole() }.wrap_err("Failed to free console")?;
    Ok(())
}

fn enable_ansi_support() -> eyre::Result<()> {
    let handle = get_console_output_handle().wrap_err("Failed to get console output handle")?;
    let mut mode = CONSOLE_MODE::default();
    unsafe { GetConsoleMode(handle, &mut mode) }.wrap_err("Failed to get console mode")?;
    unsafe { SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) }
        .wrap_err("Failed to set console mode")?;
    Ok(())
}

fn rebind_std_handles_to_console() -> eyre::Result<()> {
    let prev_out = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }.unwrap_or_default();
    let prev_err = unsafe { GetStdHandle(STD_ERROR_HANDLE) }.unwrap_or_default();
    let prev_in = unsafe { GetStdHandle(STD_INPUT_HANDLE) }.unwrap_or_default();

    let conout = unsafe {
        CreateFileW(
            w!("CONOUT$"),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .wrap_err("Failed to open CONOUT$")?;

    unsafe { SetStdHandle(STD_OUTPUT_HANDLE, conout) }
        .wrap_err("Failed to set STDOUT to CONOUT$")?;
    unsafe { SetStdHandle(STD_ERROR_HANDLE, conout) }
        .wrap_err("Failed to set STDERR to CONOUT$")?;

    close_previous_handle(prev_out, conout);
    close_previous_handle(prev_err, conout);

    if let Ok(conin) = unsafe {
        CreateFileW(
            w!("CONIN$"),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    } {
        let _ = unsafe { SetStdHandle(STD_INPUT_HANDLE, conin) };
        close_previous_handle(prev_in, conin);
    }

    Ok(())
}

fn unbind_and_close_std_handles_for_detach() {
    let out = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }.unwrap_or_default();
    let err = unsafe { GetStdHandle(STD_ERROR_HANDLE) }.unwrap_or_default();
    let inp = unsafe { GetStdHandle(STD_INPUT_HANDLE) }.unwrap_or_default();

    let _ = unsafe { SetStdHandle(STD_OUTPUT_HANDLE, HANDLE::default()) };
    let _ = unsafe { SetStdHandle(STD_ERROR_HANDLE, HANDLE::default()) };
    let _ = unsafe { SetStdHandle(STD_INPUT_HANDLE, HANDLE::default()) };

    close_handle(out);
    close_handle(err);
    close_handle(inp);
}

fn close_previous_handle(previous: HANDLE, replacement: HANDLE) {
    if previous != replacement {
        close_handle(previous);
    }
}

fn close_handle(handle: HANDLE) {
    if !handle.is_invalid() {
        let _ = unsafe { CloseHandle(handle) };
    }
}
