use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct SiteShowArgs {}

impl SiteShowArgs {
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
