#![windows_subsystem = "windows"]

fn main() -> eyre::Result<()> {
    attach_parent_console_for_cli_invocation();
    cm::main()
}

#[cfg(windows)]
fn attach_parent_console_for_cli_invocation() {
    if std::env::args_os().len() > 1
        && cm::windows_utils::console::get_console_output_handle().is_err()
    {
        let _ = cm::windows_utils::console::console_attach(u32::MAX);
    }
}

#[cfg(not(windows))]
fn attach_parent_console_for_cli_invocation() {}
