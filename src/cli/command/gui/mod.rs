use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use std::ffi::OsString;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug, Default)]
pub struct GuiArgs {}

impl GuiArgs {
    /// # Errors
    ///
    /// Returns an error if the GUI runtime cannot be created or the GUI fails to run.
    pub fn invoke(self) -> eyre::Result<()> {
        crate::gui::run_gui()
    }
}

impl ToArgs for GuiArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![]
    }
}
