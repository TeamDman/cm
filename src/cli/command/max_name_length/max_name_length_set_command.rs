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
    pub fn invoke(self) -> eyre::Result<()> {
        crate::MaxNameLength::set_to(self.length)?;
        println!("Setting max name length to: {}", self.length);
        Ok(())
    }
}

impl ToArgs for MaxNameLengthSetArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![self.length.to_string().into()]
    }
}
