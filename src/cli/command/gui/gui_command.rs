use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;
use crate::cli::command::gui::GuiArgs;

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct GuiCommandArgs {}

impl GuiCommandArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        // Build a blocking runtime and run the GUI
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async { crate::gui::run_gui().await })
    }
}

impl ToArgs for GuiCommandArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![]
    }
}
