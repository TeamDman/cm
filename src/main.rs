#![windows_subsystem = "windows"]

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    attach_parent_console_if_needed();
    cm::main()
}

#[cfg(windows)]
fn attach_parent_console_if_needed() {
    if cm::windows_utils::console::get_console_output_handle().is_err() {
        let _ = cm::windows_utils::console::console_attach(u32::MAX);
    }
}

#[cfg(not(windows))]
fn attach_parent_console_if_needed() {}
