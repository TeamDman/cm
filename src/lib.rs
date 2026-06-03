#![deny(clippy::disallowed_methods)]

pub mod app_home;
pub mod cache;
pub mod cli;
pub mod egui;
pub mod image_processing;
pub mod inputs;
pub mod max_name_length;
pub mod product_search;
pub mod reactor;
pub mod recent_input_paths;
pub mod recent_output_dirs;
pub mod rename_rules;
pub mod session_id;
pub mod site_id;
pub mod tracing;
pub mod user_id;

use crate::cli::Cli;
pub use max_name_length::*;
pub use session_id::*;
pub use site_id::*;
pub use user_id::*;

// Entrypoint matching the pattern in teamy-rust-cli
/// # Errors
/// Returns an error if CLI parsing fails or if tracing initialization fails or if the invoked command fails.
pub fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli: Cli = figue::from_std_args().unwrap();
    let app_home = crate::app_home::AppHome::resolve()?;

    // Initialize tracing based on global args (debug and --json/--log-file)
    crate::tracing::init_tracing(
        cli.global_args.log_level(),
        &cli.global_args.json_log_behaviour(),
    )?;

    cli.invoke(&app_home)?;
    Ok(())
}
