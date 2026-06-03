use crate::cli::command::rename_rule::RenameRuleAddArgs;
use crate::cli::command::rename_rule::RenameRuleListArgs;
use crate::cli::command::rename_rule::RenameRulePathArgs;
use crate::cli::command::rename_rule::RenameRuleRemoveArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct RenameRuleArgs {
    #[facet(args::subcommand)]
    pub command: RenameRuleCommand,
}

#[derive(Facet, Clone, Arbitrary, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum RenameRuleCommand {
    /// Add a rename rule
    Add(RenameRuleAddArgs),

    /// List rules
    List(RenameRuleListArgs),

    /// Print the path the rename rules live in
    Path(RenameRulePathArgs),

    /// Remove rule by id or --all
    Remove(RenameRuleRemoveArgs),
}

impl RenameRuleArgs {
    /// # Errors
    ///
    /// Returns an error if the rename rule subcommand fails.
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.invoke()
    }
}

impl RenameRuleCommand {
    /// # Errors
    ///
    /// Returns an error if the rename rule command fails.
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            RenameRuleCommand::Add(args) => args.invoke(),
            RenameRuleCommand::List(args) => args.invoke(),
            RenameRuleCommand::Path(args) => args.invoke(),
            RenameRuleCommand::Remove(args) => args.invoke(),
        }
    }
}
