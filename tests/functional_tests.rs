use c7_mcp::handler::{Context7Client, Context7Tool};
use rmcp::handler::server::ServerHandler;

#[tokio::test]
async fn test_server_initialization() {
    let server = Context7Tool::new(None);

    // Test get_info returns valid server information
    let info = server.get_info();
    assert_eq!(info.server_info.name, "c7-mcp");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));

    // Test server capabilities
    assert!(info.capabilities.tools.is_some());
}

#[tokio::test]
async fn test_client_initialization() {
    // Test client can be created without API key
    let _client1 = Context7Client::new_with_base_url(None, "https://context7.com/api".to_string());

    // Test client can be created with API key
    let _client2 = Context7Client::new_with_base_url(
        Some("test-key".to_string()),
        "https://context7.com/api".to_string(),
    );

    // Should not panic during initialization
}

#[tokio::test]
async fn test_server_info_structure() {
    let server = Context7Tool::new(None);
    let info = server.get_info();

    // Check protocol version is set
    assert!(!format!("{:?}", info.protocol_version).is_empty());

    // Check server info structure
    assert!(!info.server_info.name.is_empty());
    assert!(!info.server_info.version.is_empty());

    // Check capabilities structure
    assert!(info.capabilities.tools.is_some());
}
