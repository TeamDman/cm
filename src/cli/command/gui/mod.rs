use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

#[derive(Args, Arbitrary, Clone, PartialEq, Debug, Default)]
pub struct GuiArgs {}

impl GuiArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        // Create a dedicated runtime and run the GUI
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async { crate::gui::run_gui() })
    }
}

impl ToArgs for GuiArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![]
    }
}
