use crate::cli::command::site::site_reset_command::SiteResetArgs;
use crate::cli::command::site::site_set_command::SiteSetArgs;
use crate::cli::command::site::site_show_command::SiteShowArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;
use std::ffi::OsString;

#[derive(Subcommand, Clone, Arbitrary, PartialEq, Debug)]
pub enum SiteCommand {
    /// Show the current site (or default)
    Show(SiteShowArgs),

    /// Set the active site by id
    Set(SiteSetArgs),

    /// Reset the site to the default value and write it to the config file
    Reset(SiteResetArgs),
}

impl SiteCommand {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            SiteCommand::Show(args) => args.invoke(),
            SiteCommand::Set(args) => args.invoke(),
            SiteCommand::Reset(args) => args.invoke(),
        }
    }
}

impl ToArgs for SiteCommand {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            SiteCommand::Show(a) => {
                args.push("show".into());
                args.extend(a.to_args());
            }
            SiteCommand::Set(a) => {
                args.push("set".into());
                args.extend(a.to_args());
            }
            SiteCommand::Reset(a) => {
                args.push("reset".into());
                args.extend(a.to_args());
            }
        }
        args
    }
}
