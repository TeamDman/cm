use std::ffi::OsString;

pub trait ToArgs {
    fn to_args(&self) -> Vec<OsString>;
}
