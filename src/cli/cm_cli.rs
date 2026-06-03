use crate::app_home::AppHome;
use crate::cli::command::Command;
use crate::cli::global_args::GlobalArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::FigueBuiltins;
use figue::{self as args};

#[derive(Facet, Arbitrary, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct Cli {
    #[facet(flatten)]
    pub global_args: GlobalArgs,

    #[facet(flatten)]
    #[arbitrary(default)]
    pub builtins: FigueBuiltins,

    #[facet(args::subcommand)]
    pub command: Option<Command>,
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
        self.command.unwrap_or_default().invoke(app_home)
    }
}
