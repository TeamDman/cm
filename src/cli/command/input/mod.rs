pub mod input_command;

use crate::cli::command::input::input_command::InputCommand;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct InputArgs {
    #[clap(subcommand)]
    pub command: InputCommand,
}

impl InputArgs {
    /// # Errors
    ///
    /// Returns an error if the input subcommand fails.
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.invoke()
    }
}

impl ToArgs for InputArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}
