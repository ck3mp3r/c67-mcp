use anyhow::Result;
use reqwest::Client;
use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{serve_server, RequestContext, RoleServer};
use rmcp::transport::io::stdio;
use rmcp::transport::async_rw::AsyncRwTransport;
use rmcp::{ErrorData as McpError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info};

const CONTEXT7_API_BASE_URL: &str = "https://context7.com/api";
const MINIMUM_TOKENS: u32 = 1000;
const DEFAULT_TOKENS: u32 = 5000;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "totalSnippets")]
    pub total_snippets: Option<i32>,
    #[serde(rename = "trustScore")]
    pub trust_score: Option<i32>,
    pub versions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub error: Option<String>,
}

pub struct Context7Client {
    client: Client,
    api_key: Option<String>,
    base_url: String,
}

impl Context7Client {
    fn new(api_key: Option<String>) -> Self {
        Self::new_with_base_url(api_key, CONTEXT7_API_BASE_URL.to_string())
    }

    pub fn new_with_base_url(api_key: Option<String>, base_url: String) -> Self {
        let mut client_builder = Client::builder();

        // Configure proxy if environment variables are set
        if let Ok(proxy_url) = env::var("HTTPS_PROXY")
            .or_else(|_| env::var("https_proxy"))
            .or_else(|_| env::var("HTTP_PROXY"))
            .or_else(|_| env::var("http_proxy"))
            && let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                client_builder = client_builder.proxy(proxy);
            }

        Self {
            client: client_builder.build().unwrap_or_else(|_| Client::new()),
            api_key,
            base_url,
        }
    }

    pub async fn search_libraries(&self, query: &str) -> Result<SearchResponse> {
        let url = format!("{}/v1/search", self.base_url);

        let mut request = self.client.get(&url).query(&[("query", query)]);

        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let search_response: SearchResponse = response.json().await?;
                Ok(search_response)
            }
            reqwest::StatusCode::TOO_MANY_REQUESTS => Ok(SearchResponse {
                results: vec![],
                error: Some(
                    "Rate limited due to too many requests. Please try again later.".to_string(),
                ),
            }),
            reqwest::StatusCode::UNAUTHORIZED => Ok(SearchResponse {
                results: vec![],
                error: Some("Unauthorized. Please check your API key.".to_string()),
            }),
            status => Ok(SearchResponse {
                results: vec![],
                error: Some(format!(
                    "Failed to search libraries. Error code: {}",
                    status
                )),
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

        let mut request = self
            .client
            .get(&url)
            .query(&[("tokens", tokens.to_string())])
            .query(&[("type", "txt")]);

        if let Some(topic) = topic {
            request = request.query(&[("topic", topic)]);
        }

        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let text = response.text().await?;
                if text.is_empty() || text == "No content available" || text == "No context data available" {
                    Ok(None)
                } else {
                    Ok(Some(text))
                }
            }
            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                Ok(Some("Rate limited due to too many requests. Please try again later.".to_string()))
            }
            reqwest::StatusCode::NOT_FOUND => {
                Ok(Some("The library you are trying to access does not exist. Please try with a different library ID.".to_string()))
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                Ok(Some("Unauthorized. Please check your API key.".to_string()))
            }
            status => {
                Ok(Some(format!("Failed to fetch documentation. Error code: {}", status)))
            }
        }
    }
}

pub struct Context7Tool {
    client: Arc<Context7Client>,
}

impl Context7Tool {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Arc::new(Context7Client::new(api_key)),
        }
    }

    pub fn new_with_client(client: Context7Client) -> Self {
        Self {
            client: Arc::new(client),
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
                && snippets != -1 {
                    parts.push(format!("- Code Snippets: {}", snippets));
                }

            if let Some(trust_score) = result.trust_score
                && trust_score != -1 {
                    parts.push(format!("- Trust Score: {}", trust_score));
                }

            if let Some(versions) = &result.versions
                && !versions.is_empty() {
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
                name: "c7-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                title: None,
                website_url: None,
            },
            instructions: None,
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut tools = Vec::new();

        // Create input schemas as Arc<Map<String, Value>>
        let mut resolve_schema = Map::new();
        resolve_schema.insert("type".to_string(), Value::String("object".to_string()));
        let mut resolve_props = Map::new();
        let mut library_name_prop = Map::new();
        library_name_prop.insert("type".to_string(), Value::String("string".to_string()));
        library_name_prop.insert("description".to_string(), Value::String("Library name to search for and retrieve a Context7-compatible library ID.".to_string()));
        resolve_props.insert("libraryName".to_string(), Value::Object(library_name_prop));
        resolve_schema.insert("properties".to_string(), Value::Object(resolve_props));
        resolve_schema.insert("required".to_string(), Value::Array(vec![Value::String("libraryName".to_string())]));

        tools.push(Tool {
            name: "resolve-library-id".into(),
            description: Some("Resolves a package/product name to a Context7-compatible library ID and returns a list of matching libraries.".into()),
            input_schema: Arc::new(resolve_schema),
            output_schema: None,
            annotations: None,
            icons: None,
            title: None,
        });

        let mut docs_schema = Map::new();
        docs_schema.insert("type".to_string(), Value::String("object".to_string()));
        let mut docs_props = Map::new();
        
        let mut library_id_prop = Map::new();
        library_id_prop.insert("type".to_string(), Value::String("string".to_string()));
        library_id_prop.insert("description".to_string(), Value::String("Exact Context7-compatible library ID (e.g., '/mongodb/docs', '/vercel/next.js', '/supabase/supabase', '/vercel/next.js/v14.3.0-canary.87') retrieved from 'resolve-library-id' or directly from user query in the format '/org/project' or '/org/project/version'.".to_string()));
        docs_props.insert("context7CompatibleLibraryID".to_string(), Value::Object(library_id_prop));
        
        let mut tokens_prop = Map::new();
        tokens_prop.insert("type".to_string(), Value::String("number".to_string()));
        tokens_prop.insert("description".to_string(), Value::String("Maximum number of tokens of documentation to retrieve (default: 5000). Higher values provide more context but consume more tokens.".to_string()));
        docs_props.insert("tokens".to_string(), Value::Object(tokens_prop));
        
        let mut topic_prop = Map::new();
        topic_prop.insert("type".to_string(), Value::String("string".to_string()));
        topic_prop.insert("description".to_string(), Value::String("Topic to focus documentation on (e.g., 'hooks', 'routing').".to_string()));
        docs_props.insert("topic".to_string(), Value::Object(topic_prop));
        
        docs_schema.insert("properties".to_string(), Value::Object(docs_props));
        docs_schema.insert("required".to_string(), Value::Array(vec![Value::String("context7CompatibleLibraryID".to_string())]));

        tools.push(Tool {
            name: "get-library-docs".into(),
            description: Some("Fetches up-to-date documentation for a library. You must call 'resolve-library-id' first to obtain the exact Context7-compatible library ID required to use this tool.".into()),
            input_schema: Arc::new(docs_schema),
            output_schema: None,
            annotations: None,
            icons: None,
            title: None,
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
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "resolve-library-id" => {
                let library_name = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("libraryName"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_params("Missing libraryName parameter", None))?;

                debug!("Searching for library: {}", library_name);

                match self.client.search_libraries(library_name).await {
                    Ok(response) => {
                        if let Some(error) = &response.error {
                            Ok(CallToolResult::success(vec![Content::text(error)]))
                        } else {
                            let results_text = format_search_results(&response);
                            let text = format!("Available Libraries (top matches):\n\nEach result includes:\n- Library ID: Context7-compatible identifier (format: /org/project)\n- Name: Library or package name\n- Description: Short summary\n- Code Snippets: Number of available code examples\n- Trust Score: Authority indicator\n- Versions: List of versions if available. Use one of those versions if and only if the user explicitly provides a version in their query.\n\nFor best results, select libraries based on name match, trust score, snippet coverage, and relevance to your use case.\n\n----------\n\n{}", results_text);
                            Ok(CallToolResult::success(vec![Content::text(text)]))
                        }
                    }
                    Err(e) => {
                        error!("Failed to search libraries: {}", e);
                        let text = format!("Failed to retrieve library documentation data from Context7: {}", e);
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
                    .ok_or_else(|| McpError::invalid_params("Missing context7CompatibleLibraryID parameter", None))?;

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
            _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
        }
    }
}

pub async fn run_server(api_key: Option<String>) -> Result<()> {
    let handler = Context7Tool::new(api_key);
    
    info!("Context7 MCP server starting with stdio transport");
    
    // Use the stdio transport with async read/write  
    let (stdin, stdout) = stdio();
    let transport = AsyncRwTransport::new(stdin, stdout);
    
    serve_server(handler, transport).await?;
    
    Ok(())
}