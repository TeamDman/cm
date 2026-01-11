use crate::SESSION_ID;
use crate::SITE_ID;
use crate::USER_ID;
use crate::cache::CacheEntry;
use crate::cli::command::search::search_result_ok::SearchResultOk;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::ValueEnum;
use facet_pretty::FacetPretty;
use std::ffi::OsString;
use tracing::Instrument;
use tracing::Level;
use tracing::debug;
use tracing::field::Empty;
use tracing::info;
use tracing::span;

#[derive(ValueEnum, Arbitrary, Clone, PartialEq, Debug)]
pub enum OutputFormat {
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
#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct SearchArgs {
    /// Query to search for
    pub query: Option<String>,
    /// SKU to search for
    #[clap(long)]
    pub sku: Option<String>,
    /// Bypass the cache and fetch fresh data
    #[clap(long)]
    #[arbitrary(value = false)]
    pub no_cache: bool,
    /// Output mode: auto|json|pretty
    #[clap(long, value_enum, default_value_t = OutputFormat::Auto)]
    pub output: OutputFormat,
}

impl SearchArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        // Build a blocking runtime and perform a simple HTTP GET to the Searchspring endpoint.
        tokio::runtime::Runtime::new()?.block_on(async move {
            let result = self.search().await?;
            match match self.output {
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
                    println!("{}", json);
                }
            }

            eyre::Ok(())
        })?;

        Ok(())
    }

    /// Perform a search against the Searchspring API.
    /// <https://docs.searchspring.com/reference/get-search>
    pub async fn search(&self) -> eyre::Result<SearchResultOk> {
        let query = self.query.as_deref().unwrap_or_default();
        let site_id = SITE_ID.as_str().to_string();
        let user = USER_ID.as_uuid().to_string();
        let session = SESSION_ID.as_uuid().to_string();
        let url = format!(
            "https://{}.a.searchspring.io/api/search/search.json",
            site_id
        );
        let git_rev = option_env!("GIT_REVISION").unwrap_or("unknown");
        let user_agent = format!(
            "{} v{} (rev {}) (+https://github.com/TeamDman/cm)",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            git_rev
        );
        let span = span!(
            Level::DEBUG,
            "search_command",
            query,
            url,
            site_id,
            git_rev,
            user_agent,
            user,
            session,
            response_status = Empty,
        );
        let mut query_params = vec![
            // ("lastViewed", "664269"),
            ("userId", user.as_str()),
            ("siteId", site_id.as_str()),
            ("sessionId", session.as_str()),
            ("bgfilter.searchspring_exclude", "No"),
            ("bgfilter.visibility", "Catalog"),
            ("bgfilter.ss_advisor_exclusive", "0"),
            ("bgfilter.ss_category", "Shop"),
            ("bgfilter.ss_customer_visibility", "0"),
            ("q", query),
            ("noBeacon", "true"),
            ("ajaxCatalog", "Snap"),
            ("resultsFormat", "native"),
            ("includedFacets", "none"),
            ("page", "1"),
            ("resultsPerPage", "8"),
        ];
        if let Some(sku) = &self.sku {
            query_params.push(("filter.sku", sku.as_str()));
        }

        // Build full URL with query params for caching
        let full_url = reqwest::Url::parse_with_params(&url, &query_params)?;
        let full_url_str = full_url.to_string();

        // Check cache first (unless --no-cache is specified)
        let cache_entry = CacheEntry::for_url(&full_url_str);
        if !self.no_cache
            && let Some(cached_body) = cache_entry.read()?
        {
            info!(
                "Using cached search result for query '{}' sku '{}'",
                query,
                self.sku.as_deref().unwrap_or("")
            );
            return Self::parse_response(&cached_body);
        }

        info!(
            "Performing search for query '{}' sku '{}'",
            query,
            self.sku.as_deref().unwrap_or("")
        );
        let _guard = span.enter();
        let resp = reqwest::Client::new()
            .get(&url)
            .header(reqwest::header::USER_AGENT, user_agent)
            .query(&query_params)
            .send()
            .instrument(span.clone())
            .await?;

        let status = resp.status();
        span.record("response_status", status.as_u16());
        debug!(
            content_length = resp.content_length().unwrap_or(0),
            "Received response"
        );

        // Get the response text for caching
        let body = resp.text().await?;

        // Cache the response
        cache_entry.write(&full_url_str, &body)?;

        Self::parse_response(&body)
    }

    /// Parse the JSON response body into SearchResultOk.
    fn parse_response(body: &str) -> eyre::Result<SearchResultOk> {
        facet_json::from_str(body).map_err(|e| eyre::eyre!("Failed to parse response: {}", e))
    }
}

impl ToArgs for SearchArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut rtn = vec![];
        if let Some(q) = &self.query {
            rtn.push(OsString::from(q));
        }
        if let Some(sku) = &self.sku {
            rtn.push(OsString::from("--sku"));
            rtn.push(OsString::from(sku));
        }
        if self.no_cache {
            rtn.push(OsString::from("--no-cache"));
        }
        if self.output != OutputFormat::Auto {
            rtn.push(OsString::from("--output"));
            rtn.push(OsString::from(self.output.to_string()));
        }
        rtn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn to_args_includes_output_when_set() {
        let args = SearchArgs {
            query: None,
            sku: None,
            no_cache: false,
            output: OutputFormat::Json,
        };
        let v = args.to_args();
        assert!(
            v.windows(2)
                .any(|w| w == [OsString::from("--output"), OsString::from("json")])
        );
    }
}
