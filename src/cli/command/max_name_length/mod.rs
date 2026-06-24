pub mod max_name_length_command;
pub mod max_name_length_reset_command;
pub mod max_name_length_set_command;
pub mod max_name_length_show_command;

use crate::app_home::AppHome;
use crate::cli::command::max_name_length::max_name_length_command::MaxNameLengthCommand;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use std::ffi::OsString;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
pub struct MaxNameLengthArgs {
    #[facet(args::subcommand)]
    pub command: MaxNameLengthCommand,
}

impl MaxNameLengthArgs {
    /// # Errors
    ///
    /// Returns an error if the max name length subcommand fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        self.command.invoke(app_home)
    }
}

impl ToArgs for MaxNameLengthArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}
