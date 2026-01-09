use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;
use std::sync::atomic::Ordering;

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct MaxNameLengthShowArgs {}

impl MaxNameLengthShowArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        println!(
            "Max name length: {}",
            crate::MAX_NAME_LENGTH.load(Ordering::SeqCst)
        );
        Ok(())
    }
}

impl ToArgs for MaxNameLengthShowArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
