pub mod rename_rule_command;

use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
pub use rename_rule_command::*;
use std::ffi::OsString;

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct RenameRuleArgs {
    #[clap(subcommand)]
    pub command: RenameRuleCommand,
}

impl RenameRuleArgs {
    /// # Errors
    ///
    /// Returns an error if the rename rule subcommand fails.
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.invoke()
    }
}

impl ToArgs for RenameRuleArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}
