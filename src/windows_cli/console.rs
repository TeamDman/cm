#[cfg(windows)]
use color_eyre::owo_colors::OwoColorize;
#[cfg(windows)]
use eyre::Context;
#[cfg(windows)]
use std::sync::OnceLock;
#[cfg(windows)]
use std::sync::atomic::AtomicBool;
#[cfg(windows)]
use std::sync::atomic::AtomicU64;
#[cfg(windows)]
use std::sync::atomic::Ordering;
#[cfg(windows)]
use tracing::error;
#[cfg(windows)]
use tracing::info;

/// Hide the default console window for GUI launches that did not inherit one.
///
/// When `cm` is launched from an existing terminal, the inherited console stays
/// attached and normal Ctrl+C behavior is preserved. When it is launched by
/// opening the executable directly, the default console window is detached.
///
/// # Errors
///
/// Returns an error if detaching from the default console fails.
#[cfg(windows)]
pub fn hide_default_console_or_attach_ctrl_handler() -> eyre::Result<()> {
    if is_inheriting_console() {
        attach_ctrl_c_handler()?;
        return Ok(());
    }

    console_detach()
}

/// Hide the default console window for GUI launches that did not inherit one.
///
/// # Errors
///
/// Never returns an error on non-Windows platforms.
#[cfg(not(windows))]
pub fn hide_default_console_or_attach_ctrl_handler() -> eyre::Result<()> {
    Ok(())
}

#[cfg(windows)]
static CLOSE_REQUESTED: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static LAST_CTRL_C_MILLIS: AtomicU64 = AtomicU64::new(0);
#[cfg(windows)]
static GUI_CONTEXT: OnceLock<eframe::egui::Context> = OnceLock::new();
#[cfg(windows)]
static REACTOR_MARSHALLER: OnceLock<windows_reactor::UiMarshaller> = OnceLock::new();

/// Register the GUI context so the console handler can wake the app loop.
#[cfg(windows)]
pub fn register_gui_context(ctx: &eframe::egui::Context) {
    let _ = GUI_CONTEXT.set(ctx.clone());
}

#[cfg(not(windows))]
pub fn register_gui_context(_ctx: &eframe::egui::Context) {}

/// Register the Reactor marshaller so the console handler can wake that UI loop.
#[cfg(windows)]
pub fn register_reactor_marshaller(marshaller: &windows_reactor::UiMarshaller) {
    let _ = REACTOR_MARSHALLER.set(marshaller.clone());
}

#[cfg(not(windows))]
pub fn register_reactor_marshaller(_marshaller: &()) {}

/// Returns whether Ctrl+C requested a graceful close, clearing the request.
#[cfg(windows)]
#[must_use]
pub fn take_close_requested() -> bool {
    CLOSE_REQUESTED.swap(false, Ordering::SeqCst)
}

#[cfg(not(windows))]
#[must_use]
pub fn take_close_requested() -> bool {
    false
}

#[cfg(windows)]
fn current_tick_count_millis() -> u64 {
    unsafe { windows::Win32::System::SystemInformation::GetTickCount64() }
}

#[cfg(windows)]
fn is_inheriting_console() -> bool {
    use windows::Win32::System::Console::GetConsoleProcessList;

    let mut pids = [0_u32; 4];
    let count = unsafe { GetConsoleProcessList(pids.as_mut_slice()) };
    count > 1
}

#[cfg(windows)]
fn console_detach() -> eyre::Result<()> {
    use windows::Win32::System::Console::FreeConsole;

    unbind_and_close_std_handles_for_detach();
    unsafe { FreeConsole() }.wrap_err("Failed to free console")?;
    Ok(())
}

#[cfg(windows)]
fn attach_ctrl_c_handler() -> windows::core::Result<()> {
    unsafe {
        windows::Win32::System::Console::SetConsoleCtrlHandler(Some(ctrl_c_handler), true)?;
    }
    info!("Attached Ctrl+C handler to console");
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn ctrl_c_handler(ctrl_type: u32) -> windows::core::BOOL {
    use windows::Win32::System::Console::CTRL_BREAK_EVENT;
    use windows::Win32::System::Console::CTRL_C_EVENT;
    use windows::Win32::System::Console::CTRL_CLOSE_EVENT;
    use windows::Win32::System::Console::CTRL_LOGOFF_EVENT;
    use windows::Win32::System::Console::CTRL_SHUTDOWN_EVENT;

    match ctrl_type {
        CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT
        | CTRL_SHUTDOWN_EVENT => {
            eprintln!("{}", "^C".red());
            let now = current_tick_count_millis();
            let last = LAST_CTRL_C_MILLIS.swap(now, Ordering::SeqCst);
            if last != 0 && now.saturating_sub(last) <= 1_000 {
                eprintln!("{}", "Second Ctrl+C received, exiting immediately.".red());
                std::process::exit(130);
            }
            error!(
                "Ctrl+C received, requesting graceful shutdown. Press again within 1 second to force quit."
            );
            CLOSE_REQUESTED.store(true, Ordering::SeqCst);
            if let Some(ctx) = GUI_CONTEXT.get() {
                ctx.request_repaint();
            }
            if let Some(marshaller) = REACTOR_MARSHALLER.get() {
                let _ = marshaller.dispatch(windows_reactor::request_ui_rerender_on_ui_thread);
            }

            windows::core::BOOL(1)
        }
        _ => windows::core::BOOL(0),
    }
}

#[cfg(windows)]
fn unbind_and_close_std_handles_for_detach() {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::GetStdHandle;
    use windows::Win32::System::Console::STD_ERROR_HANDLE;
    use windows::Win32::System::Console::STD_INPUT_HANDLE;
    use windows::Win32::System::Console::STD_OUTPUT_HANDLE;
    use windows::Win32::System::Console::SetStdHandle;

    let stdout_handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }.unwrap_or_default();
    let stderr_handle = unsafe { GetStdHandle(STD_ERROR_HANDLE) }.unwrap_or_default();
    let stdin_handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) }.unwrap_or_default();

    let _ = unsafe { SetStdHandle(STD_OUTPUT_HANDLE, HANDLE::default()) };
    let _ = unsafe { SetStdHandle(STD_ERROR_HANDLE, HANDLE::default()) };
    let _ = unsafe { SetStdHandle(STD_INPUT_HANDLE, HANDLE::default()) };

    if !stdout_handle.is_invalid() {
        let _ = unsafe { CloseHandle(stdout_handle) };
    }
    if !stderr_handle.is_invalid() {
        let _ = unsafe { CloseHandle(stderr_handle) };
    }
    if !stdin_handle.is_invalid() {
        let _ = unsafe { CloseHandle(stdin_handle) };
    }
}
