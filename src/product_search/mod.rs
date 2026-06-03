mod search_request;
pub mod search_result_ok;

use facet_pretty::FacetPretty;
pub use search_request::*;
pub use search_result_ok::*;

/// Run product search in-process and return the same pretty text used by the CLI.
///
/// # Errors
///
/// Returns an error string if the Tokio runtime cannot be created or the search request fails.
pub fn search_pretty(query: &str, sku: &str) -> Result<String, String> {
    let request = SearchRequest {
        query: trimmed_optional(query).map(str::to_string),
        sku: trimmed_optional(sku).map(str::to_string),
        no_cache: false,
    };

    if request.query.is_none() && request.sku.is_none() {
        return Ok("Enter a query or SKU to search.".to_string());
    }

    tokio::runtime::Runtime::new()
        .map_err(|error| format!("Failed to start search runtime: {error}"))?
        .block_on(async move {
            request
                .await
                .map(|result| format!("{}", result.pretty()))
                .map_err(|error| format!("Search failed: {error}"))
        })
}

fn trimmed_optional(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value) }
}
