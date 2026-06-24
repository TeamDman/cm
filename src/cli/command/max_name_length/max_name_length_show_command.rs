use crate::MaxNameLength;
use crate::app_home::AppHome;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use std::ffi::OsString;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
pub struct MaxNameLengthShowArgs {}

impl MaxNameLengthShowArgs {
    /// # Errors
    ///
    /// Returns an error if loading the max name length fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        println!(
            "Max name length: {}",
            MaxNameLength::load(app_home)?.as_usize()
        );
        Ok(())
    }
}

impl ToArgs for MaxNameLengthShowArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
