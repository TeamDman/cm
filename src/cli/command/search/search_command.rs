use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use tracing::debug;
use std::ffi::OsString;

/// Search for a query (stub)
#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct SearchArgs {
    /// Query to search for
    pub query: String,
}

impl SearchArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        // Build a blocking runtime and perform a simple HTTP GET to the Searchspring endpoint.
        let site = crate::SITE_ID.as_str().to_string();
        let query = self.query.clone();
        let user = crate::USER_ID.as_uuid().to_string();
        let session = crate::SESSION_ID.as_uuid().to_string();

        let rt = tokio::runtime::Runtime::new().map_err(|e| eyre::eyre!(e))?;
        rt.block_on(async move {
            let client = reqwest::Client::new();
            let url = format!("https://{}.a.searchspring.io/api/search/search.json", site);

            let ua = format!("{} v{} (+https://github.com/TeamDman/cm)", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            debug!(ua, "Using User-Agent header");
            let resp = client
                .get(&url)
                .header(reqwest::header::USER_AGENT, ua)
                .query(&[
                        ("lastViewed", "664269"),
                        ("userId", user.as_str()),
                        ("domain", "https://www.creativememories.ca/shop-search.html?q=paper+pack&view=products"),
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
                    ])
                .send()
                .await
                .map_err(|e| eyre::eyre!(e))?;

            let status = resp.status();
            let text = resp.text().await.map_err(|e| eyre::eyre!(e))?;

            println!("Search request status: {}", status);
            let preview = if text.len() > 200 {
                &text[..200]
            } else {
                &text
            };
            println!("Response preview:\n{}", preview);

            Ok::<(), eyre::Report>(())
        })?;

        Ok(())
    }
}

impl ToArgs for SearchArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![self.query.clone().into()]
    }
}
