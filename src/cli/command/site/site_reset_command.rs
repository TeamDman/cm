use crate::SiteId;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

/// Reset the site to the default value and persist it to the config file
#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct SiteResetArgs {}

impl SiteResetArgs {
    /// # Errors
    ///
    /// Returns an error if resetting the site fails.
    pub fn invoke(self) -> eyre::Result<()> {
        SiteId::set_to(SiteId::DEFAULT)?;
        println!("Reset site to default: {}", SiteId::DEFAULT);
        Ok(())
    }
}

impl ToArgs for SiteResetArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
