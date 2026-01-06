use crate::MaxNameLength;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

/// Reset the max name length to the default value and persist it to the config file
#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct MaxNameLengthResetArgs {}

impl MaxNameLengthResetArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        MaxNameLength::set_to(MaxNameLength::DEFAULT)?;
        println!("Reset max name length to default: {}", MaxNameLength::DEFAULT);
        Ok(())
    }
}

impl ToArgs for MaxNameLengthResetArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
