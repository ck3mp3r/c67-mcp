use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ureq::tls::{RootCerts, TlsConfig};
use ureq::{Agent, Error};

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
    pub fn new(api_key: Option<String>, insecure: bool) -> Self {
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

        let api_key = self.api_key.clone();
        let query = query.to_string();
        let insecure = self.insecure;
        let result = tokio::task::spawn_blocking(move || {
            let agent = if insecure {
                // Create agent with empty certificate store to bypass verification (insecure mode)
                let tls_config = TlsConfig::builder()
                    .root_certs(RootCerts::Specific(Arc::new(vec![])))
                    .build();

                Agent::config_builder()
                    .tls_config(tls_config)
                    .build()
                    .new_agent()
            } else {
                ureq::agent()
            };

            let mut request = agent.get(&url).query("query", &query);

            if let Some(api_key) = api_key {
                request = request.header("Authorization", &format!("Bearer {}", api_key));
            }

            request.call()
        })
        .await?;

        match result {
            Ok(mut response) => {
                let search_response: SearchResponse = response.body_mut().read_json()?;
                Ok(search_response)
            }
            Err(Error::StatusCode(429)) => Ok(SearchResponse {
                results: vec![],
                error: Some(
                    "Rate limited due to too many requests. Please try again later.".to_string(),
                ),
            }),
            Err(Error::StatusCode(401)) => Ok(SearchResponse {
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

        let api_key = self.api_key.clone();
        let topic = topic.map(|s| s.to_string());
        let insecure = self.insecure;

        let result = tokio::task::spawn_blocking(move || {
            let agent = if insecure {
                // Create agent with empty certificate store to bypass verification (insecure mode)
                let tls_config = TlsConfig::builder()
                    .root_certs(RootCerts::Specific(Arc::new(vec![])))
                    .build();

                Agent::config_builder()
                    .tls_config(tls_config)
                    .build()
                    .new_agent()
            } else {
                ureq::agent()
            };

            let mut request = agent
                .get(&url)
                .query("tokens", tokens.to_string())
                .query("type", "txt");

            if let Some(topic) = topic {
                request = request.query("topic", &topic);
            }

            if let Some(api_key) = api_key {
                request = request.header("Authorization", &format!("Bearer {}", api_key));
            }

            request = request.header("X-Context7-Source", "mcp-server");

            request.call()
        })
        .await?;

        match result {
            Ok(mut response) => {
                let text = response.body_mut().read_to_string()?;
                if text.is_empty() || text == "No content available" || text == "No context data available" {
                    Ok(None)
                } else {
                    Ok(Some(text))
                }
            }
            Err(Error::StatusCode(429)) => {
                Ok(Some("Rate limited due to too many requests. Please try again later.".to_string()))
            }
            Err(Error::StatusCode(404)) => {
                Ok(Some("The library you are trying to access does not exist. Please try with a different library ID.".to_string()))
            }
            Err(Error::StatusCode(401)) => {
                Ok(Some("Unauthorized. Please check your API key.".to_string()))
            }
            Err(e) => {
                Ok(Some(format!("Failed to fetch documentation: {}", e)))
            }
        }
    }
}
