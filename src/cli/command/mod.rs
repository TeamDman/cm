pub mod search;
pub mod site;

use crate::cli::command::search::search_command::SearchArgs;
use crate::cli::command::site::SiteArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;
use std::ffi::OsString;

#[derive(Subcommand, Arbitrary, PartialEq, Debug)]
pub enum Command {
    /// Site related commands
    Site(SiteArgs),

    /// Search (stub)
    Search(SearchArgs),
}

impl Command {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            Command::Site(args) => args.invoke(),
            Command::Search(args) => args.invoke(),
        }
    }
}

impl ToArgs for Command {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            Command::Site(site_args) => {
                args.push("site".into());
                args.extend(site_args.to_args());
            }
            Command::Search(search_args) => {
                args.push("search".into());
                args.extend(search_args.to_args());
            }
        }
        args
    }
}
