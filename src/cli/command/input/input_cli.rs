use crate::cli::command::input::InputAddArgs;
use crate::cli::command::input::InputListArgs;
use crate::cli::command::input::InputRemoveArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct InputArgs {
    #[facet(args::subcommand)]
    pub command: InputCommand,
}

#[derive(Facet, Clone, Arbitrary, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum InputCommand {
    /// Add input paths (glob expands files; matched paths are canonicalized and persisted)
    Add(InputAddArgs),

    /// List persisted input paths
    List(InputListArgs),

    /// Remove persisted input paths matching a glob
    Remove(InputRemoveArgs),
}

impl InputArgs {
    /// # Errors
    ///
    /// Returns an error if the input subcommand fails.
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.invoke()
    }
}

impl InputCommand {
    /// # Errors
    ///
    /// Returns an error if the input command fails.
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            InputCommand::Add(args) => args.invoke(),
            InputCommand::List(args) => args.invoke(),
            InputCommand::Remove(args) => args.invoke(),
        }
    }
}
