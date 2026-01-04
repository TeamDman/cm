pub mod clean;
pub mod gui;
pub mod input;
pub mod search;
pub mod site;

use crate::cli::command::clean::clean_command::CleanArgs;
use crate::cli::command::gui::GuiArgs;
use crate::cli::command::input::InputArgs;
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

    /// Search
    Search(SearchArgs),

    /// Inputs persistent list (add/list/remove)
    Input(InputArgs),

    /// Launch a graphical user interface
    Gui(GuiArgs),

    /// Clean cached API responses
    Clean(CleanArgs),
}
impl Default for Command {
    fn default() -> Self {
        Command::Gui(Default::default())
    }
}

impl Command {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            Command::Site(args) => args.invoke(),
            Command::Search(args) => args.invoke(),
            Command::Input(args) => args.invoke(),
            Command::Gui(args) => args.invoke(),
            Command::Clean(args) => args.invoke(),
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
            Command::Input(input_args) => {
                args.push("input".into());
                args.extend(input_args.to_args());
            }
            Command::Gui(gui_args) => {
                args.push("gui".into());
                args.extend(gui_args.to_args());
            }
            Command::Clean(clean_args) => {
                args.push("clean".into());
                args.extend(clean_args.to_args());
            }
        }
        args
    }
}
