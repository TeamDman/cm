use crate::app_home::AppHome;
use crate::cli::command::clean::CleanArgs;
use crate::cli::command::gui::GuiArgs;
use crate::cli::command::input::InputArgs;
use crate::cli::command::max_name_length::MaxNameLengthArgs;
use crate::cli::command::search::SearchArgs;
use crate::cli::command::site::SiteArgs;
use arbitrary::Arbitrary;
use facet::Facet;

#[derive(Facet, Arbitrary, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum Command {
    /// Site related commands
    Site(SiteArgs),

    /// Max name length commands
    MaxNameLength(MaxNameLengthArgs),

    /// Search
    Search(SearchArgs),

    /// Inputs persistent list (add/list/remove)
    Input(InputArgs),

    /// Manage rename rules
    RenameRule(crate::cli::command::rename_rule::RenameRuleArgs),

    /// Launch a graphical user interface
    Gui(GuiArgs),

    /// Clean cached API responses
    Clean(CleanArgs),
}

impl Default for Command {
    fn default() -> Self {
        Command::Gui(GuiArgs::default())
    }
}

impl Command {
    /// # Errors
    ///
    /// Returns an error if the command fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        match self {
            Command::Site(args) => args.invoke(),
            Command::MaxNameLength(args) => args.invoke(app_home),
            Command::Search(args) => args.invoke(),
            Command::Input(args) => args.invoke(),
            Command::RenameRule(args) => args.invoke(),
            Command::Gui(args) => args.invoke(),
            Command::Clean(args) => args.invoke(),
        }
    }
}
