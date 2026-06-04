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
pub mod windows_cli;

use crate::cli::Cli;
use ::tracing::info;
pub use max_name_length::*;
pub use session_id::*;
pub use site_id::*;
pub use user_id::*;

/// Version string combining package version and git revision.
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (rev ",
    env!("GIT_REVISION"),
    ")"
);

// Entrypoint matching the pattern in teamy-rust-cli
/// # Errors
///
/// Returns an error if CLI parsing fails or if tracing initialization fails or if the invoked command fails.
///
/// # Panics
///
/// Panics if the CLI schema is invalid (should never happen with correct code).
pub fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli: Cli = figue::Driver::new(
        figue::builder::<Cli>()
            .expect("schema should be valid")
            .cli(move |cli| cli.args_os(std::env::args_os().skip(1)).strict())
            .help(move |help| {
                help.version(VERSION)
                    .include_implementation_source_file(true)
                    .include_implementation_git_url("TeamDman/cm", env!("GIT_REVISION"))
            })
            .build(),
    )
    .run()
    .unwrap();
    let app_home = crate::app_home::AppHome::resolve()?;

    // Initialize tracing based on global args (debug and --json/--log-file)
    crate::tracing::init_tracing(
        cli.global_args.log_level(),
        &cli.global_args.json_log_behaviour(),
    )?;

    info!("Starting cm version {}", VERSION);
    cli.invoke(&app_home)?;
    Ok(())
}
