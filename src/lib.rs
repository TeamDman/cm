#![deny(clippy::disallowed_methods)]

pub mod app_home;
pub mod cache;
pub mod cli;
pub mod gui;
pub mod inputs;
pub mod session_id;
pub mod site_id;
pub mod tracing;
pub mod user_id;

use crate::cli::Cli;
use clap::CommandFactory;
use clap::FromArgMatches;
pub use session_id::*;
pub use site_id::*;
pub use user_id::*;

// Entrypoint matching the pattern in teamy-rust-cli
pub fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::command();
    let cli = Cli::from_arg_matches(&cli.get_matches())?;

    // Initialize tracing based on global args (debug and --json/--log-file)
    crate::tracing::init_tracing(
        cli.global_args.log_level(),
        cli.global_args.json_log_behaviour(),
    )?;

    cli.invoke()?;
    Ok(())
}
