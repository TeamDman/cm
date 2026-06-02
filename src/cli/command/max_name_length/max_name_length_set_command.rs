use crate::app_home::AppHome;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

/// Set the max name length
#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct MaxNameLengthSetArgs {
    /// Length value to set
    pub length: usize,
}

impl MaxNameLengthSetArgs {
    /// # Errors
    ///
    /// Returns an error if setting the max name length fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        crate::MaxNameLength::set_to(app_home, self.length)?;
        println!("Setting max name length to: {}", self.length);
        Ok(())
    }
}

impl ToArgs for MaxNameLengthSetArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![self.length.to_string().into()]
    }
}
