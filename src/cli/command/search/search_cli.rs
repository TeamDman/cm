use crate::product_search::SearchRequest;
use arbitrary::Arbitrary;
use facet::Facet;
use facet_pretty::FacetPretty;
use figue::{self as args};

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug, Default)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum OutputFormat {
    #[default]
    Auto,
    Json,
    Pretty,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Json => write!(f, "json"),
            Self::Pretty => write!(f, "pretty"),
        }
    }
}

/// Search for a query
#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct SearchArgs {
    #[facet(flatten, default)]
    pub request: SearchRequest,
    /// Output mode: auto|json|pretty
    #[facet(args::named, default)]
    pub output: OutputFormat,
}

impl SearchArgs {
    /// # Errors
    ///
    /// Returns an error if the search fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let SearchArgs { request, output } = self;

        // Build a blocking runtime and perform a simple HTTP GET to the Searchspring endpoint.
        tokio::runtime::Runtime::new()?.block_on(async move {
            let result = request.await?;
            match match output {
                OutputFormat::Auto => {
                    if atty::is(atty::Stream::Stdout) {
                        OutputFormat::Pretty
                    } else {
                        OutputFormat::Json
                    }
                }
                other => other,
            } {
                OutputFormat::Auto => unreachable!("output was resolved from Auto earlier"),
                OutputFormat::Pretty => {
                    println!("{}", result.pretty());
                }
                OutputFormat::Json => {
                    let json = facet_json::to_string(&result)
                        .map_err(|e| eyre::eyre!("Failed to serialize result: {}", e))?;
                    println!("{json}");
                }
            }

            eyre::Ok(())
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figue::ToArgs;
    use std::ffi::OsString;

    #[test]
    fn to_args_includes_output_when_set() {
        let args = SearchArgs {
            request: SearchRequest {
                query: None,
                sku: None,
                no_cache: false,
            },
            output: OutputFormat::Json,
        };
        let v = args.to_args().expect("search args should serialize");
        assert!(
            v.windows(2)
                .any(|w| w == [OsString::from("--output"), OsString::from("json")])
        );
    }
}
