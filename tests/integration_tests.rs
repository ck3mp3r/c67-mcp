use c67_mcp::handler::{Context7Client, SearchResponse, SearchResult};
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_context7_client_search_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/search"))
        .and(query_param("query", "nix"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [
                {
                    "id": "/nixos/nix",
                    "title": "Nix",
                    "description": "The Nix package manager",
                    "totalSnippets": 1241,
                    "trustScore": 9.0,
                    "versions": ["2.18.0", "2.17.0"]
                }
            ],
            "error": null
        })))
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(None, mock_server.uri(), false);
    let result = client.search_libraries("nix").await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].id, "/nixos/nix");
    assert_eq!(response.results[0].title, "Nix");
    assert_eq!(response.results[0].total_snippets, Some(1241));
    assert_eq!(response.results[0].trust_score, Some(9.0));
    assert!(response.error.is_none());
}

#[tokio::test]
async fn test_context7_client_search_with_api_key() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/search"))
        .and(query_param("query", "react"))
        .and(header("authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [],
            "error": null
        })))
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(
        Some("test-api-key".to_string()),
        mock_server.uri(),
        false,
    );
    let result = client.search_libraries("react").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_context7_client_search_rate_limited() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(None, mock_server.uri(), false);
    let result = client.search_libraries("test").await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.results.is_empty());
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("Rate limited"));
}

#[tokio::test]
async fn test_context7_client_fetch_docs_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/nixos/nix"))
        .and(query_param("tokens", "5000"))
        .and(query_param("type", "txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "# Getting Started with Nix\n\nInstall Nix: `curl -L https://nixos.org/nix/install | sh`"
        ))
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(None, mock_server.uri(), false);
    let result = client
        .fetch_library_documentation("/nixos/nix", Some(5000), None)
        .await;

    assert!(result.is_ok());
    let docs = result.unwrap();
    assert!(docs.is_some());
    let content = docs.unwrap();
    assert!(content.contains("Getting Started with Nix"));
    assert!(content.contains("curl -L https://nixos.org/nix/install"));
}

#[tokio::test]
async fn test_context7_client_fetch_docs_with_topic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/nixos/nix"))
        .and(query_param("tokens", "3000"))
        .and(query_param("type", "txt"))
        .and(query_param("topic", "installation"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("Installation documentation content"),
        )
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(None, mock_server.uri(), false);
    let result = client
        .fetch_library_documentation("/nixos/nix", Some(3000), Some("installation"))
        .await;

    assert!(result.is_ok());
    let docs = result.unwrap();
    assert!(docs.is_some());
    assert!(docs.unwrap().contains("Installation documentation"));
}

#[tokio::test]
async fn test_context7_client_fetch_docs_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/nonexistent/library"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(None, mock_server.uri(), false);
    let result = client
        .fetch_library_documentation("/nonexistent/library", None, None)
        .await;

    assert!(result.is_ok());
    let docs = result.unwrap();
    assert!(docs.is_some());
    assert!(docs.unwrap().contains("does not exist"));
}

#[tokio::test]
async fn test_context7_client_fetch_docs_empty_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/empty/library"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(None, mock_server.uri(), false);
    let result = client
        .fetch_library_documentation("/empty/library", None, None)
        .await;

    assert!(result.is_ok());
    let docs = result.unwrap();
    assert!(docs.is_none());
}

#[tokio::test]
async fn test_library_id_leading_slash_handling() {
    let mock_server = MockServer::start().await;

    // Should strip leading slash from library ID
    Mock::given(method("GET"))
        .and(path("/v1/nixos/nix")) // No leading slash in the path
        .respond_with(ResponseTemplate::new(200).set_body_string("content"))
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(None, mock_server.uri(), false);
    let result = client
        .fetch_library_documentation("/nixos/nix", None, None)
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[tokio::test]
async fn test_token_limits_enforcement() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/test/lib"))
        .respond_with(|req: &wiremock::Request| {
            let tokens = req
                .url
                .query_pairs()
                .find(|(key, _)| key == "tokens")
                .map(|(_, value)| value.parse::<u32>().unwrap_or(0))
                .unwrap_or(0);

            // Should enforce minimum of 1000 tokens
            assert!(
                tokens >= 1000,
                "Tokens should be at least 1000, got: {}",
                tokens
            );

            ResponseTemplate::new(200).set_body_string("content")
        })
        .mount(&mock_server)
        .await;

    let client = Context7Client::new_with_base_url(None, mock_server.uri(), false);

    // Test with very low token count - should be increased to minimum
    let result = client
        .fetch_library_documentation("/test/lib", Some(100), None)
        .await;
    assert!(result.is_ok());

    // Test with no token count - should use default
    let result = client
        .fetch_library_documentation("/test/lib", None, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_search_response_formatting() {
    use c67_mcp::handler::format_search_results;

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
