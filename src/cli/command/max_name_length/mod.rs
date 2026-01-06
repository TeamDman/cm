pub mod max_name_length_command;
pub mod max_name_length_reset_command;
pub mod max_name_length_set_command;
pub mod max_name_length_show_command;

use crate::cli::command::max_name_length::max_name_length_command::MaxNameLengthCommand;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

#[derive(Args, Arbitrary, PartialEq, Debug)]
pub struct MaxNameLengthArgs {
    #[clap(subcommand)]
    pub command: MaxNameLengthCommand,
}

impl MaxNameLengthArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.invoke()
    }
}

impl ToArgs for MaxNameLengthArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}
