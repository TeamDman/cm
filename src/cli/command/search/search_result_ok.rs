//! Strongly-typed representation of the Searchspring search response.
//! See: <https://docs.searchspring.com/reference/get-search?siteId=4y9u7l>

use facet::Facet;

/// Pagination info from the search response.
/// Note: Searchspring returns these as integers in the JSON.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Pagination {
    #[facet(rename = "totalResults")]
    pub total_results: Option<i64>,
    pub begin: Option<i64>,
    pub end: Option<i64>,
    #[facet(rename = "currentPage")]
    pub current_page: Option<i64>,
    #[facet(rename = "totalPages")]
    pub total_pages: Option<i64>,
    #[facet(rename = "previousPage")]
    pub previous_page: Option<i64>,
    #[facet(rename = "nextPage")]
    pub next_page: Option<i64>,
    #[facet(rename = "perPage")]
    pub per_page: Option<i64>,
    #[facet(rename = "defaultPerPage")]
    pub default_per_page: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SortingOption {
    pub field: Option<String>,
    pub direction: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Sorting {
    pub options: Option<Vec<SortingOption>>,
}

// Result item newtypes
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Uid(pub String);
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Sku(pub String);
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Url(pub String);
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Price(pub String);
impl Price {
    #[must_use] 
    pub fn as_f64(&self) -> Option<f64> {
        self.0.parse().ok()
    }
}

/// A result item from the search response.
/// Note: The API returns many more fields than we model here.
/// Unknown fields are captured as `extra` using `RawJson`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct ResultItem {
    pub uid: Option<Uid>,
    pub sku: Option<Sku>,
    pub name: Option<String>,
    pub url: Option<Url>,
    #[facet(rename = "addToCartUrl")]
    pub add_to_cart_url: Option<String>,
    pub price: Option<Price>,
    pub msrp: Option<String>,
    #[facet(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[facet(rename = "thumbnailImageUrl")]
    pub thumbnail_image_url: Option<String>,
    pub rating: Option<String>,
    #[facet(rename = "ratingCount")]
    pub rating_count: Option<String>,
    pub description: Option<String>,
    #[facet(rename = "stockMessage")]
    pub stock_message: Option<String>,
    pub brand: Option<String>,
    pub popularity: Option<String>,
    #[facet(rename = "intellisuggestData")]
    pub intellisuggest_data: Option<String>,
    #[facet(rename = "intellisuggestSignature")]
    pub intellisuggest_signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct FacetValue {
    pub active: Option<bool>,
    #[facet(rename = "type")]
    pub value_type: Option<String>,
    pub value: Option<String>,
    pub low: Option<String>,
    pub high: Option<String>,
    pub label: Option<String>,
    pub count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SearchFacet {
    pub field: Option<String>,
    pub label: Option<String>,
    #[facet(rename = "type")]
    pub facet_type: Option<String>,
    pub multiple: Option<String>,
    /// 0 = Expanded, 1 = Collapsed
    pub collapse: Option<i64>,
    /// 0 = not active, 1 = active
    pub facet_active: Option<i64>,
    pub values: Option<Vec<FacetValue>>,
    #[facet(rename = "hierarchyDelimiter")]
    pub hierarchy_delimiter: Option<String>,
    pub step: Option<i64>,
    /// For sliders - active range values
    pub active: Option<Vec<f64>>,
    /// For sliders - available range
    pub range: Option<Vec<f64>>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Breadcrumb {
    pub field: Option<String>,
    pub label: Option<String>,
    #[facet(rename = "filterLabel")]
    pub filter_label: Option<String>,
    #[facet(rename = "filterValue")]
    pub filter_value: Option<String>,
    /// Array of filter parameters to remove
    #[facet(rename = "removeFilters")]
    pub remove_filters: Option<Vec<String>>,
    /// Array of refine query parameters to remove
    #[facet(rename = "removeRefineQuery")]
    pub remove_refine_query: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct FilterSummary {
    pub field: Option<String>,
    pub value: Option<String>,
    pub label: Option<String>,
    #[facet(rename = "filterLabel")]
    pub filter_label: Option<String>,
    #[facet(rename = "filterValue")]
    pub filter_value: Option<String>,
}

/// Merchandising is not currently parsed due to its polymorphic nature.
/// The API returns either empty strings or arrays for various fields.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Merchandising {
    pub personalized: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct DidYouMean {
    pub query: Option<String>,
    pub highlighted: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct QueryInfo {
    #[facet(rename = "matchType")]
    pub match_type: Option<String>,
    pub original: Option<String>,
    pub corrected: Option<String>,
}

/// The top-level search result response.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SearchResultOk {
    pub pagination: Option<Pagination>,
    pub sorting: Option<Sorting>,
    #[facet(rename = "resultLayout")]
    pub result_layout: Option<String>,
    pub results: Option<Vec<ResultItem>>,
    pub facets: Option<Vec<SearchFacet>>,
    pub breadcrumbs: Option<Vec<Breadcrumb>>,
    #[facet(rename = "filterSummary")]
    pub filter_summary: Option<Vec<FilterSummary>>,
    pub merchandising: Option<Merchandising>,
    #[facet(rename = "didYouMean")]
    pub did_you_mean: Option<DidYouMean>,
    pub query: Option<QueryInfo>,
}

// Minimal test to ensure deserialization works for a very small sample response.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal() {
        // Note: totalResults is an integer in the actual API
        let raw = r#"{ "results": [{ "uid": "id-1", "name": "Item 1", "price": "9.99" }], "pagination": { "totalResults": 1 } }"#;
        let got: SearchResultOk = facet_json::from_str(raw).expect("should deserialize");
        assert!(got.results.is_some());
        let first = &got.results.unwrap()[0];
        assert_eq!(first.name.as_deref(), Some("Item 1"));
    }

    #[test]
    fn deserialize_with_facet_integers() {
        // Test that integer fields like collapse work correctly
        let raw = r#"{
            "facets": [{
                "field": "brand",
                "label": "Brand",
                "type": "list",
                "collapse": 0,
                "facet_active": 1,
                "values": []
            }],
            "results": []
        }"#;
        let got: SearchResultOk = facet_json::from_str(raw).expect("should deserialize");
        assert!(got.facets.is_some());
        let facets = got.facets.unwrap();
        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].collapse, Some(0));
        assert_eq!(facets[0].facet_active, Some(1));
    }

    #[test]
    fn deserialize_pagination_integers() {
        let raw = r#"{
            "pagination": {
                "totalResults": 4,
                "begin": 1,
                "end": 4,
                "currentPage": 1,
                "totalPages": 1,
                "previousPage": 0,
                "nextPage": 0,
                "perPage": 8,
                "defaultPerPage": 20
            },
            "results": []
        }"#;
        let got: SearchResultOk = facet_json::from_str(raw).expect("should deserialize");
        assert!(got.pagination.is_some());
        let p = got.pagination.unwrap();
        assert_eq!(p.total_results, Some(4));
        assert_eq!(p.begin, Some(1));
        assert_eq!(p.per_page, Some(8));
    }
}
