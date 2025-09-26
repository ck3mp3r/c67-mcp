use c67_mcp::handler::{Context7Client, Context7Tool};
use rmcp::handler::server::ServerHandler;

#[tokio::test]
async fn test_server_initialization() {
    let server = Context7Tool::new(None, false);

    // Test get_info returns valid server information
    let info = server.get_info();
    assert_eq!(info.server_info.name, "c67-mcp");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));

    // Test server capabilities
    assert!(info.capabilities.tools.is_some());
}

#[tokio::test]
async fn test_client_initialization() {
    // Test client can be created without API key
    let _client1 =
        Context7Client::new_with_base_url(None, "https://context7.com/api".to_string(), false);

    // Test client can be created with API key
    let _client2 = Context7Client::new_with_base_url(
        Some("test-key".to_string()),
        "https://context7.com/api".to_string(),
        false,
    );

    // Should not panic during initialization
}

#[tokio::test]
async fn test_server_info_structure() {
    let server = Context7Tool::new(None, false);
    let info = server.get_info();

    // Check protocol version is set
    assert!(!format!("{:?}", info.protocol_version).is_empty());

    // Check server info structure
    assert!(!info.server_info.name.is_empty());
    assert!(!info.server_info.version.is_empty());

    // Check capabilities structure
    assert!(info.capabilities.tools.is_some());
}

#[tokio::test]
async fn test_client_initialization_with_insecure_flag() {
    // Test client can be created without insecure flag
    let _client_secure =
        Context7Client::new_with_base_url(None, "https://context7.com/api".to_string(), false);

    // Test client can be created with insecure flag
    let _client_insecure =
        Context7Client::new_with_base_url(None, "https://context7.com/api".to_string(), true);

    // Test client with API key and insecure flag
    let _client_insecure_with_key = Context7Client::new_with_base_url(
        Some("test-key".to_string()),
        "https://context7.com/api".to_string(),
        true,
    );

    // Should not panic during initialization
}

#[tokio::test]
async fn test_server_initialization_with_insecure_flag() {
    // Test server can be created with insecure flag disabled
    let server_secure = Context7Tool::new(None, false);
    let info_secure = server_secure.get_info();
    assert_eq!(info_secure.server_info.name, "c67-mcp");

    // Test server can be created with insecure flag enabled
    let server_insecure = Context7Tool::new(None, true);
    let info_insecure = server_insecure.get_info();
    assert_eq!(info_insecure.server_info.name, "c67-mcp");

    // Test server with API key and insecure flag
    let server_insecure_with_key = Context7Tool::new(Some("test-key".to_string()), true);
    let info_key = server_insecure_with_key.get_info();
    assert_eq!(info_key.server_info.name, "c67-mcp");
}
