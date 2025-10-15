#[cfg(test)]
mod tests {
    use crate::Context7Client;

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
}
