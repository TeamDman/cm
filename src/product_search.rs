use crate::cli::command::search::search_command::OutputFormat;
use crate::cli::command::search::search_command::SearchArgs;
use facet_pretty::FacetPretty;

/// Run product search in-process and return the same pretty text used by the CLI.
///
/// This keeps UI shells from spawning `cm search` just to reuse the product
/// lookup logic.
pub fn search_pretty(query: &str, sku: &str) -> Result<String, String> {
    let query = trimmed_optional(query);
    let sku = trimmed_optional(sku);

    if query.is_none() && sku.is_none() {
        return Ok("Enter a query or SKU to search.".to_string());
    }

    let args = SearchArgs {
        query: query.map(str::to_string),
        sku: sku.map(str::to_string),
        no_cache: false,
        output: OutputFormat::Pretty,
    };

    tokio::runtime::Runtime::new()
        .map_err(|error| format!("Failed to start search runtime: {error}"))?
        .block_on(async move {
            args.search()
                .await
                .map(|result| format!("{}", result.pretty()))
                .map_err(|error| format!("Search failed: {error}"))
        })
}

fn trimmed_optional(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value) }
}
