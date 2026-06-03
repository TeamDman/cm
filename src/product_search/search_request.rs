use crate::SESSION_ID;
use crate::SITE_ID;
use crate::USER_ID;
use crate::cache::CacheEntry;
use crate::product_search::search_result_ok::SearchResultOk;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};
use std::pin::Pin;
use std::sync::LazyLock;
use tokio::sync::Mutex;
use tracing::Instrument;
use tracing::Level;
use tracing::debug;
use tracing::field::Empty;
use tracing::info;
use tracing::span;

/// Global mutex to serialize product searches (maximizes cache hits when multiple images share SKUs)
static SEARCH_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug, Default)]
#[facet(rename_all = "kebab-case")]
pub struct SearchRequest {
    /// Query to search for
    #[facet(default, args::positional)]
    pub query: Option<String>,
    /// SKU to search for
    #[facet(args::named)]
    pub sku: Option<String>,
    /// Bypass the cache and fetch fresh data
    #[facet(args::named, default)]
    #[arbitrary(value = false)]
    pub no_cache: bool,
}

impl std::future::IntoFuture for SearchRequest {
    type Output = eyre::Result<SearchResultOk>;
    type IntoFuture = Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            // Acquire mutex to serialize searches - this maximizes cache hits
            let _guard = SEARCH_MUTEX.lock().await;

            let query = self.query.unwrap_or_default();
            let sku = self.sku;
            let no_cache = self.no_cache;
            let site_id = SITE_ID.as_str().to_string();
            let user = USER_ID.as_uuid().to_string();
            let session = SESSION_ID.as_uuid().to_string();
            let url = format!("https://{site_id}.a.searchspring.io/api/search/search.json");
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
                ("q", query.as_str()),
                ("noBeacon", "true"),
                ("ajaxCatalog", "Snap"),
                ("resultsFormat", "native"),
                ("includedFacets", "none"),
                ("page", "1"),
                ("resultsPerPage", "8"),
            ];
            if let Some(sku) = &sku {
                query_params.push(("filter.sku", sku.as_str()));
            }

            // Build full URL with query params for caching
            let full_url = reqwest::Url::parse_with_params(&url, &query_params)?;
            let full_url_str = full_url.to_string();

            // Check cache first (unless --no-cache is specified)
            let cache_entry = CacheEntry::for_url(&full_url_str);
            if !no_cache && let Some(cached_body) = cache_entry.read()? {
                info!(
                    "Using cached search result for query '{}' sku '{}'",
                    query,
                    sku.as_deref().unwrap_or("")
                );
                return {
                    let body: &str = &cached_body;
                    facet_json::from_str(body)
                        .map_err(|e| eyre::eyre!("Failed to parse response: {}", e))
                };
            }

            info!(
                "Performing search for query '{}' sku '{}'",
                query,
                sku.as_deref().unwrap_or("")
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

            let body = resp.text().await?;
            cache_entry.write(&full_url_str, &body)?;

            facet_json::from_str(&body).map_err(|e| eyre::eyre!("Failed to parse response: {}", e))
        })
    }
}
