#[cfg(test)]
mod tests {
    use crate::Context7Client;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_context7_client_with_insecure_flag() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/search"))
            .and(query_param("query", "test-insecure"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {
                        "id": "/test/insecure",
                        "title": "Test Insecure Library",
                        "description": "Testing insecure TLS connections",
                        "totalSnippets": 42,
                        "trustScore": 7.5,
                        "versions": ["1.0.0"]
                    }
                ],
                "error": null
            })))
            .mount(&mock_server)
            .await;

        // Test client with insecure flag enabled - should work with mock server
        let client_insecure = Context7Client::new_with_base_url(None, mock_server.uri(), true);
        let result_insecure = client_insecure.search_libraries("test-insecure").await;

        assert!(result_insecure.is_ok());
        let response_insecure = result_insecure.unwrap();
        assert_eq!(response_insecure.results.len(), 1);
        assert_eq!(response_insecure.results[0].id, "/test/insecure");
        assert_eq!(response_insecure.results[0].title, "Test Insecure Library");

        // Test client with insecure flag disabled - should also work with mock server (HTTP)
        let client_secure = Context7Client::new_with_base_url(None, mock_server.uri(), false);
        let result_secure = client_secure.search_libraries("test-insecure").await;

        assert!(result_secure.is_ok());
        let response_secure = result_secure.unwrap();
        assert_eq!(response_secure.results.len(), 1);
        assert_eq!(response_secure.results[0].id, "/test/insecure");
    }

    #[tokio::test]
    async fn test_context7_client_fetch_docs_with_insecure_flag() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/test/insecure"))
            .and(query_param("tokens", "5000"))
            .and(query_param("type", "txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "# Insecure Connection Test\n\nThis tests insecure TLS connections for corporate environments."
            ))
            .mount(&mock_server)
            .await;

        // Test documentation fetching with insecure flag
        let client_insecure = Context7Client::new_with_base_url(None, mock_server.uri(), true);
        let result_insecure = client_insecure
            .fetch_library_documentation("/test/insecure", Some(5000), None)
            .await;

        assert!(result_insecure.is_ok());
        let docs_insecure = result_insecure.unwrap();
        assert!(docs_insecure.is_some());
        let content_insecure = docs_insecure.unwrap();
        assert!(content_insecure.contains("Insecure Connection Test"));
        assert!(content_insecure.contains("corporate environments"));

        // Test documentation fetching without insecure flag
        let client_secure = Context7Client::new_with_base_url(None, mock_server.uri(), false);
        let result_secure = client_secure
            .fetch_library_documentation("/test/insecure", Some(5000), None)
            .await;

        assert!(result_secure.is_ok());
        let docs_secure = result_secure.unwrap();
        assert!(docs_secure.is_some());
        let content_secure = docs_secure.unwrap();
        assert!(content_secure.contains("Insecure Connection Test"));
    }
}
