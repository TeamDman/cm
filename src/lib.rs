#![deny(clippy::disallowed_methods)]

pub mod app_home;
pub mod cache;
pub mod cli;
pub mod gui;
pub mod image_processing;
pub mod inputs;
pub mod max_name_length;
pub mod rename_rules;
pub mod session_id;
pub mod site_id;
pub mod tracing;
pub mod user_id;

use crate::cli::Cli;
use chrono::{DateTime, Local, Utc};
use clap::CommandFactory;
use clap::FromArgMatches;
pub use max_name_length::*;
pub use session_id::*;
pub use site_id::*;
pub use user_id::*;

/// Version string combining package version, git revision, and build time.
fn version() -> String {
    let built_at = option_env!("BUILD_TIMESTAMP_UNIX")
        .and_then(|value| value.parse::<i64>().ok())
        .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
        .map_or_else(
            || "unknown build time".to_string(),
            |timestamp| {
                timestamp
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S %Z")
                    .to_string()
            },
        );

    format!(
        "{} (rev {}, built {})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_REVISION"),
        built_at,
    )
}

// Entrypoint matching the pattern in teamy-rust-cli
/// # Errors
/// Returns an error if CLI parsing fails or if tracing initialization fails or if the invoked command fails.
pub fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let version = version();
    let cli = Cli::command().long_version(Box::leak(version.into_boxed_str()) as &'static str);
    let cli = Cli::from_arg_matches(&cli.get_matches())?;
    let app_home = crate::app_home::AppHome::resolve()?;

    // Initialize tracing based on global args (debug and --json/--log-file)
    crate::tracing::init_tracing(
        cli.global_args.log_level(),
        &cli.global_args.json_log_behaviour(),
    )?;

    cli.invoke(&app_home)?;
    Ok(())
}
