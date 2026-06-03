use crate::cli::command::site::SiteResetArgs;
use crate::cli::command::site::SiteSetArgs;
use crate::cli::command::site::SiteShowArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Arbitrary, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct SiteArgs {
    #[facet(args::subcommand)]
    pub command: SiteCommand,
}

#[derive(Facet, Clone, Arbitrary, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum SiteCommand {
    /// Show the current site (or default)
    Show(SiteShowArgs),

    /// Set the active site by id
    Set(SiteSetArgs),

    /// Reset the site to the default value and write it to the config file
    Reset(SiteResetArgs),
}

impl SiteArgs {
    /// # Errors
    ///
    /// Returns an error if the site subcommand fails.
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.invoke()
    }
}

impl SiteCommand {
    /// # Errors
    ///
    /// Returns an error if the site command fails.
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            SiteCommand::Show(args) => args.invoke(),
            SiteCommand::Set(args) => args.invoke(),
            SiteCommand::Reset(args) => args.invoke(),
        }
    }
}
