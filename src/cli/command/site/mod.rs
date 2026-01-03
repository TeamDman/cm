pub mod site_command;
pub mod site_reset_command;
pub mod site_set_command;
pub mod site_show_command;

use crate::cli::command::site::site_command::SiteCommand;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

#[derive(Args, Arbitrary, PartialEq, Debug)]
pub struct SiteArgs {
    #[clap(subcommand)]
    pub command: SiteCommand,
}

impl SiteArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.invoke()
    }
}

impl ToArgs for SiteArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}
