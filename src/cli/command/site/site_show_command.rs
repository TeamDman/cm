use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use std::ffi::OsString;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
pub struct SiteShowArgs {}

impl SiteShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    pub fn invoke(self) -> eyre::Result<()> {
        // Use the static SITE_ID for the current value
        println!("Site: {}", crate::SITE_ID.as_str());
        Ok(())
    }
}

impl ToArgs for SiteShowArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
