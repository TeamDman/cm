use crate::app_home::APP_HOME;
use crate::cli::to_args::ToArgs;
use crate::inputs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use std::ffi::OsString;

#[derive(Subcommand, Clone, Arbitrary, PartialEq, Debug)]
pub enum InputCommand {
    /// Add input paths (glob expands files; matched paths are canonicalized and persisted)
    Add(InputAddArgs),

    /// List persisted input paths
    List(InputListArgs),

    /// Remove persisted input paths matching a glob
    Remove(InputRemoveArgs),
}

impl InputCommand {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            InputCommand::Add(a) => a.invoke(),
            InputCommand::List(a) => a.invoke(),
            InputCommand::Remove(a) => a.invoke(),
        }
    }
}

impl ToArgs for InputCommand {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            InputCommand::Add(a) => {
                args.push("add".into());
                args.extend(a.to_args());
            }
            InputCommand::List(a) => {
                args.push("list".into());
                args.extend(a.to_args());
            }
            InputCommand::Remove(a) => {
                args.push("remove".into());
                args.extend(a.to_args());
            }
        }
        args
    }
}

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct InputAddArgs {
    /// Glob pattern to add (file paths matched will be canonicalized and stored)
    pub pattern: String,
}

impl InputAddArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        let added = inputs::add_from_glob(&APP_HOME, &self.pattern)?;
        for p in &added {
            println!("Added: {}", p.display());
        }
        if added.is_empty() {
            println!("No matching paths were found for '{}'.", self.pattern);
        }
        Ok(())
    }
}

impl ToArgs for InputAddArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![OsString::from(self.pattern.clone())]
    }
}

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct InputListArgs {}

impl InputListArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        let list = inputs::load_inputs(&APP_HOME)?;
        for p in list {
            println!("{}", p.display());
        }
        Ok(())
    }
}

impl ToArgs for InputListArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![]
    }
}

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct InputRemoveArgs {
    /// Glob pattern for paths to remove
    pub pattern: String,
}

impl InputRemoveArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        let removed = inputs::remove_from_glob(&APP_HOME, &self.pattern)?;
        for p in &removed {
            println!("Removed: {}", p.display());
        }
        if removed.is_empty() {
            println!("No persisted inputs matched '{}'.", self.pattern);
        }
        Ok(())
    }
}

impl ToArgs for InputRemoveArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![OsString::from(self.pattern.clone())]
    }
}
