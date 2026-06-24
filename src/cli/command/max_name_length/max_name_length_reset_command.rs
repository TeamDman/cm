use crate::MaxNameLength;
use crate::app_home::AppHome;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use std::ffi::OsString;

/// Reset the max name length to the default value and persist it to the config file
#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
pub struct MaxNameLengthResetArgs {}

impl MaxNameLengthResetArgs {
    /// # Errors
    ///
    /// Returns an error if resetting the max name length fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        MaxNameLength::set_to(app_home, MaxNameLength::DEFAULT)?;
        println!(
            "Reset max name length to default: {}",
            MaxNameLength::DEFAULT
        );
        Ok(())
    }
}

impl ToArgs for MaxNameLengthResetArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
