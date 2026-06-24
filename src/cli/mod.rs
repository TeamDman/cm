pub mod command;
pub mod global_args;
pub mod json_log_behaviour;
pub mod to_args;

use crate::app_home::AppHome;
use crate::cli::command::Command;
use crate::cli::global_args::GlobalArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use figue::FigueBuiltins;
use std::ffi::OsString;

#[derive(Facet, Arbitrary, Debug)]
pub struct Cli {
    #[facet(flatten)]
    pub global_args: GlobalArgs,

    #[facet(flatten)]
    #[arbitrary(default)]
    pub builtins: FigueBuiltins,

    #[facet(args::subcommand, default)]
    #[arbitrary(default)]
    pub command: Command,
}

impl PartialEq for Cli {
    fn eq(&self, other: &Self) -> bool {
        self.global_args == other.global_args && self.command == other.command
    }
}

impl Cli {
    /// # Errors
    ///
    /// Returns an error if the CLI command fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        self.command.invoke(app_home)
    }
}

impl ToArgs for Cli {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        args.extend(self.global_args.to_args());
        args.extend(self.command.to_args());
        args
    }
}
