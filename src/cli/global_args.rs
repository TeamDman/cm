use crate::cli::json_log_behaviour::JsonLogBehaviour;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Default, Arbitrary, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct GlobalArgs {
    /// Enable debug logging
    #[facet(args::named, default)]
    pub debug: bool,

    /// Emit structured JSON logs alongside stderr output.
    /// If set, logs are written to the given path.
    #[facet(args::named)]
    pub log_file: Option<String>,
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

    /// Get the JSON log behaviour based on the `--log-file` argument.
    #[must_use]
    pub fn json_log_behaviour(&self) -> JsonLogBehaviour {
        match &self.log_file {
            None => JsonLogBehaviour::None,
            Some(s) => JsonLogBehaviour::Some(s.into()),
        }
    }
}
