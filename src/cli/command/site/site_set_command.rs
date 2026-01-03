use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

/// Set the active site by id
#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct SiteSetArgs {
    /// Site identifier to set
    pub id: String,
}

impl SiteSetArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        // Persist the selection to disk so next runs pick it up
        crate::SiteId::set_to(&self.id)?;
        println!("Setting site to: {}", self.id);
        Ok(())
    }
}

impl ToArgs for SiteSetArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![self.id.clone().into()]
    }
}
