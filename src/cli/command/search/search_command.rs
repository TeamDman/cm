use crate::SESSION_ID;
use crate::SITE_ID;
use crate::USER_ID;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;
use tracing::Instrument;
use tracing::Level;
use tracing::debug;
use tracing::field::Empty;
use tracing::span;

/// Search for a query (stub)
#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct SearchArgs {
    /// Query to search for
    pub query: Option<String>,
    /// SKU to search for
    #[clap(long)]
    pub sku: Option<String>,
}

impl SearchArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        // Build a blocking runtime and perform a simple HTTP GET to the Searchspring endpoint.
        tokio::runtime::Runtime::new()?.block_on(async move {
            let text = self.search().await?;
            println!("{}", text);
            eyre::Ok(())
        })?;

        Ok(())
    }

    /// Perform a search against the Searchspring API.
    /// <https://docs.searchspring.com/reference/get-search>
    pub async fn search(&self) -> eyre::Result<String> {
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
            ("lastViewed", "664269"),
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
        let _guard = span.enter();
        let resp = reqwest::Client::new()
            .get(&url)
            .header(reqwest::header::USER_AGENT, user_agent)
            .query(&query_params)
            .send()
            .instrument(span.clone())
            .await?;

        let status = resp.status();
        span.record("response_status", &status.as_u16());
        let text = resp.text().await?;
        debug!(length_bytes = text.len(), "Received response");
        Ok(text)
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
        rtn
    }
}
