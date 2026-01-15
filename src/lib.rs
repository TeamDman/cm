#![deny(clippy::disallowed_methods)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::unused_async,
    missing_debug_implementations,
    unfulfilled_lint_expectations,
    clippy::struct_field_names,
    clippy::items_after_statements,
    clippy::match_same_arms,
    clippy::struct_excessive_bools,
    clippy::large_enum_variant,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::assigning_clones,
    clippy::trivially_copy_pass_by_ref,
    clippy::manual_let_else,
    clippy::format_push_string
)]

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
use clap::CommandFactory;
use clap::FromArgMatches;
pub use max_name_length::*;
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
