use crate::cli::command::max_name_length::max_name_length_reset_command::MaxNameLengthResetArgs;
use crate::cli::command::max_name_length::max_name_length_set_command::MaxNameLengthSetArgs;
use crate::cli::command::max_name_length::max_name_length_show_command::MaxNameLengthShowArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;
use std::ffi::OsString;

#[derive(Subcommand, Clone, Arbitrary, PartialEq, Debug)]
pub enum MaxNameLengthCommand {
    /// Show the current max name length
    Show(MaxNameLengthShowArgs),

    /// Set the max name length
    Set(MaxNameLengthSetArgs),

    /// Reset the max name length to the default value and write it to the config file
    Reset(MaxNameLengthResetArgs),
}

impl MaxNameLengthCommand {
    /// # Errors
    ///
    /// Returns an error if the max name length command fails.
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            MaxNameLengthCommand::Show(args) => args.invoke(),
            MaxNameLengthCommand::Set(args) => args.invoke(),
            MaxNameLengthCommand::Reset(args) => args.invoke(),
        }
    }
}

impl ToArgs for MaxNameLengthCommand {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            MaxNameLengthCommand::Show(a) => {
                args.push("show".into());
                args.extend(a.to_args());
            }
            MaxNameLengthCommand::Set(a) => {
                args.push("set".into());
                args.extend(a.to_args());
            }
            MaxNameLengthCommand::Reset(a) => {
                args.push("reset".into());
                args.extend(a.to_args());
            }
        }
        args
    }
}
