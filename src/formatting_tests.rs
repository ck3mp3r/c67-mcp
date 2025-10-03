#[cfg(test)]
mod tests {
    use crate::{format_search_results, SearchResponse, SearchResult};

    #[tokio::test]
    async fn test_search_response_formatting() {
        // Test empty results
        let empty_response = SearchResponse {
            results: vec![],
            error: None,
        };
        let formatted = format_search_results(&empty_response);
        assert_eq!(
            formatted,
            "No documentation libraries found matching your query."
        );

        // Test with results
        let response = SearchResponse {
            results: vec![
                SearchResult {
                    id: "/test/lib1".to_string(),
                    title: "Test Library 1".to_string(),
                    description: "A test library".to_string(),
                    total_snippets: Some(100),
                    trust_score: Some(8.0),
                    versions: Some(vec!["1.0.0".to_string(), "2.0.0".to_string()]),
                },
                SearchResult {
                    id: "/test/lib2".to_string(),
                    title: "Test Library 2".to_string(),
                    description: "Another test library".to_string(),
                    total_snippets: Some(-1), // Should be filtered out
                    trust_score: Some(-1.0),  // Should be filtered out
                    versions: None,
                },
            ],
            error: None,
        };

        let formatted = format_search_results(&response);

        // Check first library
        assert!(formatted.contains("Test Library 1"));
        assert!(formatted.contains("/test/lib1"));
        assert!(formatted.contains("Code Snippets: 100"));
        assert!(formatted.contains("Trust Score: 8.0"));
        assert!(formatted.contains("Versions: 1.0.0, 2.0.0"));

        // Check second library
        assert!(formatted.contains("Test Library 2"));
        assert!(formatted.contains("/test/lib2"));
        assert!(!formatted.contains("Code Snippets: -1")); // Should be filtered out
        assert!(!formatted.contains("Trust Score: -1.0")); // Should be filtered out

        // Check separator
        assert!(formatted.contains("----------"));
    }
}