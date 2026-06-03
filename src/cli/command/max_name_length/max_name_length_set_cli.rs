use crate::app_home::AppHome;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

/// Set the max name length
#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct MaxNameLengthSetArgs {
    /// Length value to set
    #[facet(args::positional)]
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
