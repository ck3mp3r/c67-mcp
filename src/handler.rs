use anyhow::Result;
use rmcp::{
    RoleServer, ServiceExt,
    handler::server::ServerHandler,
    model::*,
    serde_json::{Map, Value},
    service::RequestContext,
    transport,
};
use rustls::{
    DigitallySignedStruct, SignatureScheme,
    pki_types::{ServerName, UnixTime},
};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info};

const CONTEXT7_API_BASE_URL: &str = "https://context7.com/api";
const MINIMUM_TOKENS: u32 = 1000;
const DEFAULT_TOKENS: u32 = 5000;

/// Insecure certificate verifier that accepts all certificates (for corporate MITM)
#[derive(Debug)]
struct InsecureVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "totalSnippets")]
    pub total_snippets: Option<i32>,
    #[serde(rename = "trustScore")]
    pub trust_score: Option<f64>,
    pub versions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub error: Option<String>,
}

pub struct Context7Client {
    api_key: Option<String>,
    base_url: String,
    insecure: bool,
}

impl Context7Client {
    fn new(api_key: Option<String>, insecure: bool) -> Self {
        Self::new_with_base_url(api_key, CONTEXT7_API_BASE_URL.to_string(), insecure)
    }

    pub fn new_with_base_url(api_key: Option<String>, base_url: String, insecure: bool) -> Self {
        Self {
            api_key,
            base_url,
            insecure,
        }
    }

    pub async fn search_libraries(&self, query: &str) -> Result<SearchResponse> {
        let url = format!("{}/v1/search", self.base_url);

        // Use tokio::task::spawn_blocking to run synchronous ureq in async context
        let api_key = self.api_key.clone();
        let query = query.to_string();
        let insecure = self.insecure;
        let result = tokio::task::spawn_blocking(move || {
            // Ensure crypto provider is installed for rustls
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

            let agent = if insecure {
                use rustls::ClientConfig;
                use std::sync::Arc;

                let mut config = ClientConfig::builder()
                    .with_root_certificates(rustls::RootCertStore::empty())
                    .with_no_client_auth();

                // Create a custom verifier that accepts all certificates (insecure)
                config
                    .dangerous()
                    .set_certificate_verifier(Arc::new(InsecureVerifier));

                ureq::AgentBuilder::new()
                    .tls_config(Arc::new(config))
                    .build()
            } else {
                ureq::agent()
            };

            let mut request = agent.get(&url).query("query", &query);

            if let Some(api_key) = api_key {
                request = request.set("Authorization", &format!("Bearer {}", api_key));
            }

            request.call()
        })
        .await?;

        match result {
            Ok(response) => {
                let search_response: SearchResponse = response.into_json()?;
                Ok(search_response)
            }
            Err(ureq::Error::Status(429, _)) => Ok(SearchResponse {
                results: vec![],
                error: Some(
                    "Rate limited due to too many requests. Please try again later.".to_string(),
                ),
            }),
            Err(ureq::Error::Status(401, _)) => Ok(SearchResponse {
                results: vec![],
                error: Some("Unauthorized. Please check your API key.".to_string()),
            }),
            Err(e) => Ok(SearchResponse {
                results: vec![],
                error: Some(format!("Failed to search libraries: {}", e)),
            }),
        }
    }

    pub async fn fetch_library_documentation(
        &self,
        library_id: &str,
        tokens: Option<u32>,
        topic: Option<&str>,
    ) -> Result<Option<String>> {
        let library_id = library_id.strip_prefix('/').unwrap_or(library_id);
        let url = format!("{}/v1/{}", self.base_url, library_id);

        let tokens = tokens.unwrap_or(DEFAULT_TOKENS).max(MINIMUM_TOKENS);

        // Use tokio::task::spawn_blocking to run synchronous ureq in async context
        let api_key = self.api_key.clone();
        let topic = topic.map(|s| s.to_string());
        let insecure = self.insecure;

        let result = tokio::task::spawn_blocking(move || {
            // Ensure crypto provider is installed for rustls
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

            let agent = if insecure {
                use rustls::ClientConfig;
                use std::sync::Arc;

                let mut config = ClientConfig::builder()
                    .with_root_certificates(rustls::RootCertStore::empty())
                    .with_no_client_auth();

                // Create a custom verifier that accepts all certificates (insecure)
                config
                    .dangerous()
                    .set_certificate_verifier(Arc::new(InsecureVerifier));

                ureq::AgentBuilder::new()
                    .tls_config(Arc::new(config))
                    .build()
            } else {
                ureq::agent()
            };

            let mut request = agent
                .get(&url)
                .query("tokens", &tokens.to_string())
                .query("type", "txt");

            if let Some(topic) = topic {
                request = request.query("topic", &topic);
            }

            if let Some(api_key) = api_key {
                request = request.set("Authorization", &format!("Bearer {}", api_key));
            }

            request = request.set("X-Context7-Source", "mcp-server");

            request.call()
        })
        .await?;

        match result {
            Ok(response) => {
                let text = response.into_string()?;
                if text.is_empty() || text == "No content available" || text == "No context data available" {
                    Ok(None)
                } else {
                    Ok(Some(text))
                }
            }
            Err(ureq::Error::Status(429, _)) => {
                Ok(Some("Rate limited due to too many requests. Please try again later.".to_string()))
            }
            Err(ureq::Error::Status(404, _)) => {
                Ok(Some("The library you are trying to access does not exist. Please try with a different library ID.".to_string()))
            }
            Err(ureq::Error::Status(401, _)) => {
                Ok(Some("Unauthorized. Please check your API key.".to_string()))
            }
            Err(e) => {
                Ok(Some(format!("Failed to fetch documentation: {}", e)))
            }
        }
    }
}

#[derive(Clone)]
pub struct Context7Tool {
    client: Arc<Context7Client>,
}

impl Context7Tool {
    pub fn new(api_key: Option<String>, insecure: bool) -> Self {
        Self {
            client: Arc::new(Context7Client::new(api_key, insecure)),
        }
    }
}

pub fn format_search_results(response: &SearchResponse) -> String {
    if response.results.is_empty() {
        return "No documentation libraries found matching your query.".to_string();
    }

    let formatted_results: Vec<String> = response
        .results
        .iter()
        .map(|result| {
            let mut parts = vec![
                format!("- Title: {}", result.title),
                format!("- Context7-compatible library ID: {}", result.id),
                format!("- Description: {}", result.description),
            ];

            if let Some(snippets) = result.total_snippets
                && snippets != -1
            {
                parts.push(format!("- Code Snippets: {}", snippets));
            }

            if let Some(trust_score) = result.trust_score
                && trust_score >= 0.0
            {
                parts.push(format!("- Trust Score: {:.1}", trust_score));
            }

            if let Some(versions) = &result.versions
                && !versions.is_empty()
            {
                parts.push(format!("- Versions: {}", versions.join(", ")));
            }

            parts.join("\n")
        })
        .collect();

    formatted_results.join("\n----------\n")
}

impl ServerHandler for Context7Tool {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "c67-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Use this server to retrieve up-to-date documentation and code examples for any library.".to_string()),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut tools = Vec::new();

        // Create input schemas as Arc<Map<String, Value>>
        let mut resolve_schema = Map::new();
        resolve_schema.insert("type".to_string(), Value::String("object".to_string()));
        let mut resolve_props = Map::new();
        let mut library_name_prop = Map::new();
        library_name_prop.insert("type".to_string(), Value::String("string".to_string()));
        library_name_prop.insert(
            "description".to_string(),
            Value::String(
                "Library name to search for and retrieve a Context7-compatible library ID."
                    .to_string(),
            ),
        );
        resolve_props.insert("libraryName".to_string(), Value::Object(library_name_prop));
        resolve_schema.insert("properties".to_string(), Value::Object(resolve_props));
        resolve_schema.insert(
            "required".to_string(),
            Value::Array(vec![Value::String("libraryName".to_string())]),
        );

        tools.push(Tool {
            name: "resolve-library-id".into(),
            description: Some("Resolves a package/product name to a Context7-compatible library ID and returns a list of matching libraries.\n\nYou MUST call this function before 'get-library-docs' to obtain a valid Context7-compatible library ID UNLESS the user explicitly provides a library ID in the format '/org/project' or '/org/project/version' in their query.\n\nSelection Process:\n1. Analyze the query to understand what library/package the user is looking for\n2. Return the most relevant match based on:\n- Name similarity to the query (exact matches prioritized)\n- Description relevance to the query's intent\n- Documentation coverage (prioritize libraries with higher Code Snippet counts)\n- Trust score (consider libraries with scores of 7-10 more authoritative)\n\nResponse Format:\n- Return the selected library ID in a clearly marked section\n- Provide a brief explanation for why this library was chosen\n- If multiple good matches exist, acknowledge this but proceed with the most relevant one\n- If no good matches exist, clearly state this and suggest query refinements\n\nFor ambiguous queries, request clarification before proceeding with a best-guess match.".into()),
            input_schema: Arc::new(resolve_schema),
            annotations: None,
        });

        let mut docs_schema = Map::new();
        docs_schema.insert("type".to_string(), Value::String("object".to_string()));
        let mut docs_props = Map::new();

        let mut library_id_prop = Map::new();
        library_id_prop.insert("type".to_string(), Value::String("string".to_string()));
        library_id_prop.insert("description".to_string(), Value::String("Exact Context7-compatible library ID (e.g., '/mongodb/docs', '/vercel/next.js', '/supabase/supabase', '/vercel/next.js/v14.3.0-canary.87') retrieved from 'resolve-library-id' or directly from user query in the format '/org/project' or '/org/project/version'.".to_string()));
        docs_props.insert(
            "context7CompatibleLibraryID".to_string(),
            Value::Object(library_id_prop),
        );

        let mut tokens_prop = Map::new();
        tokens_prop.insert("type".to_string(), Value::String("number".to_string()));
        tokens_prop.insert("description".to_string(), Value::String("Maximum number of tokens of documentation to retrieve (default: 5000). Higher values provide more context but consume more tokens.".to_string()));
        docs_props.insert("tokens".to_string(), Value::Object(tokens_prop));

        let mut topic_prop = Map::new();
        topic_prop.insert("type".to_string(), Value::String("string".to_string()));
        topic_prop.insert(
            "description".to_string(),
            Value::String(
                "Topic to focus documentation on (e.g., 'hooks', 'routing').".to_string(),
            ),
        );
        docs_props.insert("topic".to_string(), Value::Object(topic_prop));

        docs_schema.insert("properties".to_string(), Value::Object(docs_props));
        docs_schema.insert(
            "required".to_string(),
            Value::Array(vec![Value::String(
                "context7CompatibleLibraryID".to_string(),
            )]),
        );

        tools.push(Tool {
            name: "get-library-docs".into(),
            description: Some("Fetches up-to-date documentation for a library. You must call 'resolve-library-id' first to obtain the exact Context7-compatible library ID required to use this tool, UNLESS the user explicitly provides a library ID in the format '/org/project' or '/org/project/version' in their query.".into()),
            input_schema: Arc::new(docs_schema),
            annotations: None,
        });

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        match request.name.as_ref() {
            "resolve-library-id" => {
                let library_name = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("libraryName"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_request(
                            "Missing libraryName parameter".to_string(),
                            None,
                        )
                    })?;

                debug!("Searching for library: {}", library_name);

                match self.client.search_libraries(library_name).await {
                    Ok(response) => {
                        if let Some(error) = &response.error {
                            Ok(CallToolResult::success(vec![Content::text(error)]))
                        } else {
                            let results_text = format_search_results(&response);
                            let text = format!(
                                "Available Libraries (top matches):\n\nEach result includes:\n- Library ID: Context7-compatible identifier (format: /org/project)\n- Name: Library or package name\n- Description: Short summary\n- Code Snippets: Number of available code examples\n- Trust Score: Authority indicator\n- Versions: List of versions if available. Use one of those versions if and only if the user explicitly provides a version in their query.\n\nFor best results, select libraries based on name match, trust score, snippet coverage, and relevance to your use case.\n\n----------\n\n{}",
                                results_text
                            );
                            Ok(CallToolResult::success(vec![Content::text(text)]))
                        }
                    }
                    Err(e) => {
                        error!("Failed to search libraries: {}", e);
                        error!("Error source: {:?}", e.source());
                        error!("Error kind: {:#?}", e);
                        let text = format!(
                            "Failed to retrieve library documentation data from Context7: {}",
                            e
                        );
                        Ok(CallToolResult::success(vec![Content::text(text)]))
                    }
                }
            }
            "get-library-docs" => {
                let library_id = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("context7CompatibleLibraryID"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_request(
                            "Missing context7CompatibleLibraryID parameter".to_string(),
                            None,
                        )
                    })?;

                let topic = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("topic"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let tokens = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("tokens"))
                    .and_then(|v| v.as_u64())
                    .map(|t| t as u32);

                debug!(
                    "Fetching docs for library: {}, topic: {:?}, tokens: {:?}",
                    library_id, topic, tokens
                );

                match self
                    .client
                    .fetch_library_documentation(library_id, tokens, topic.as_deref())
                    .await
                {
                    Ok(Some(documentation)) => {
                        Ok(CallToolResult::success(vec![Content::text(documentation)]))
                    }
                    Ok(None) => {
                        let text = "Documentation not found or not finalized for this library. This might have happened because you used an invalid Context7-compatible library ID. To get a valid Context7-compatible library ID, use the 'resolve-library-id' with the package name you wish to retrieve documentation for.";
                        Ok(CallToolResult::success(vec![Content::text(text)]))
                    }
                    Err(e) => {
                        error!("Failed to fetch documentation: {}", e);
                        let text = format!("Error fetching library documentation: {}", e);
                        Ok(CallToolResult::success(vec![Content::text(text)]))
                    }
                }
            }
            _ => Err(ErrorData::invalid_request(
                format!("Unknown tool: {}", request.name),
                None,
            )),
        }
    }
}

pub async fn run_server(api_key: Option<String>, insecure: bool) -> Result<()> {
    let tool = Context7Tool::new(api_key, insecure);

    if insecure {
        info!("Context7 MCP server starting with stdio transport (TLS verification disabled)");
    } else {
        info!("Context7 MCP server starting with stdio transport");
    }

    let service = tool.serve(transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
