pub mod command;
pub mod global_args;
pub mod json_log_behaviour;
pub mod to_args;

use crate::cli::command::Command;
use crate::cli::global_args::GlobalArgs;
use arbitrary::Arbitrary;
use clap::Parser;
use std::ffi::OsString;
use to_args::ToArgs;

#[derive(Parser, Arbitrary, PartialEq, Debug)]
#[clap(version)]
pub struct Cli {
    #[clap(flatten)]
    pub global_args: GlobalArgs,

    #[clap(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    /// # Errors
    ///
    /// Returns an error if the CLI command fails.
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.unwrap_or_default().invoke()
    }
}

impl ToArgs for Cli {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        args.extend(self.global_args.to_args());
        if let Some(command) = &self.command {
            args.extend(command.to_args());
        }
        args
    }
}
