use anyhow::Result;
use rmcp::{
    RoleServer, ServiceExt,
    handler::server::ServerHandler,
    model::*,
    serde_json::{Map, Value},
    service::RequestContext,
    transport,
};
use std::env;
use std::sync::Arc;

use crate::client::Context7Client;
use crate::formatting::format_search_results;

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
                title: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
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
            title: None,
            description: Some("Resolves a package/product name to a Context7-compatible library ID and returns a list of matching libraries.\n\nYou MUST call this function before 'get-library-docs' to obtain a valid Context7-compatible library ID UNLESS the user explicitly provides a library ID in the format '/org/project' or '/org/project/version' in their query.\n\nSelection Process:\n1. Analyze the query to understand what library/package the user is looking for\n2. Return the most relevant match based on:\n- Name similarity to the query (exact matches prioritized)\n- Description relevance to the query's intent\n- Documentation coverage (prioritize libraries with higher Code Snippet counts)\n- Trust score (consider libraries with scores of 7-10 more authoritative)\n\nResponse Format:\n- Return the selected library ID in a clearly marked section\n- Provide a brief explanation for why this library was chosen\n- If multiple good matches exist, acknowledge this but proceed with the most relevant one\n- If no good matches exist, clearly state this and suggest query refinements\n\nFor ambiguous queries, request clarification before proceeding with a best-guess match.".into()),
            input_schema: Arc::new(resolve_schema),
            output_schema: None,
            annotations: None,
            icons: None,
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
            title: None,
            description: Some("Fetches up-to-date documentation for a library. You must call 'resolve-library-id' first to obtain the exact Context7-compatible library ID required to use this tool, UNLESS the user explicitly provides a library ID in the format '/org/project' or '/org/project/version' in their query.".into()),
            input_schema: Arc::new(docs_schema),
            output_schema: None,
            annotations: None,
            icons: None,
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

                match self.client.search_libraries(library_name).await {
                    Ok(response) => {
                        if let Some(error) = &response.error {
                            Ok(CallToolResult::success(vec![Content::text(error)]))
                        } else {
                            let results_text = format_search_results(&response);
                            let text = format!(
                                "Available Libraries (top matches):\n\nEach result includes:\n- Library ID: Context7-compatible identifier (format: /org/project)\n- Name: Library or package name\n- Description: Short summary\n- Code Snippets: Number of available code examples\n- Trust Score: Authority indicator\n- Versions: List of versions if available. Use one of those versions if the user provides a version in their query. The format of the version is /org/project/version.\n\nFor best results, select libraries based on name match, trust score, snippet coverage, and relevance to your use case.\n\n----------\n\n{}",
                                results_text
                            );
                            Ok(CallToolResult::success(vec![Content::text(text)]))
                        }
                    }
                    Err(e) => {
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

    eprintln!("Context7 Documentation MCP Server running on stdio");

    let service = tool.serve(transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
