use crate::app_home::AppHome;
use crate::cli::command::max_name_length::MaxNameLengthResetArgs;
use crate::cli::command::max_name_length::MaxNameLengthSetArgs;
use crate::cli::command::max_name_length::MaxNameLengthShowArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Arbitrary, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct MaxNameLengthArgs {
    #[facet(args::subcommand)]
    pub command: MaxNameLengthCommand,
}

#[derive(Facet, Clone, Arbitrary, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum MaxNameLengthCommand {
    /// Show the current max name length
    Show(MaxNameLengthShowArgs),

    /// Set the max name length
    Set(MaxNameLengthSetArgs),

    /// Reset the max name length to the default value and write it to the config file
    Reset(MaxNameLengthResetArgs),
}

impl MaxNameLengthArgs {
    /// # Errors
    ///
    /// Returns an error if the max name length subcommand fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        self.command.invoke(app_home)
    }
}

impl MaxNameLengthCommand {
    /// # Errors
    ///
    /// Returns an error if the max name length command fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        match self {
            MaxNameLengthCommand::Show(args) => args.invoke(app_home),
            MaxNameLengthCommand::Set(args) => args.invoke(app_home),
            MaxNameLengthCommand::Reset(args) => args.invoke(app_home),
        }
    }
}
