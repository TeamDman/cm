use crate::cli::json_log_behaviour::JsonLogBehaviour;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use std::ffi::OsString;

#[derive(Facet, Default, Arbitrary, Clone, PartialEq, Debug)]
pub struct GlobalArgs {
    /// Enable debug logging
    #[facet(args::named, default)]
    pub debug: bool,

    /// Emit structured JSON logs alongside stderr output.
    /// Optionally specify a filename; if not provided, a timestamped filename will be generated.
    #[facet(args::named, default)]
    #[arbitrary(value = None)]
    log_file: Option<Option<String>>,
}

impl GlobalArgs {
    #[must_use]
    pub fn log_level(&self) -> tracing::Level {
        if self.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        }
    }

    #[must_use]
    pub fn json_log_behaviour(&self) -> JsonLogBehaviour {
        match &self.log_file {
            None => JsonLogBehaviour::None,
            Some(None) => JsonLogBehaviour::SomeAutomaticPath,
            Some(Some(s)) if s.is_empty() => JsonLogBehaviour::SomeAutomaticPath,
            Some(Some(s)) => JsonLogBehaviour::Some(s.into()),
        }
    }
}

impl ToArgs for GlobalArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        if self.debug {
            args.push("--debug".into());
        }
        match &self.log_file {
            None => {}
            Some(None) => {
                args.push("--log-file".into());
            }
            Some(Some(path)) if path.is_empty() => {
                args.push("--log-file".into());
            }
            Some(Some(path)) => {
                args.push("--log-file".into());
                args.push(path.into());
            }
        }
        args
    }
}
