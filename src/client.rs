use anyhow::Result;
use rustls::{
    DigitallySignedStruct, SignatureScheme,
    pki_types::{ServerName, UnixTime},
};
use serde::{Deserialize, Serialize};

const CONTEXT7_API_BASE_URL: &str = "https://context7.com/api";
const MINIMUM_TOKENS: u32 = 1000;
const DEFAULT_TOKENS: u32 = 5000;

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
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

            let agent = if insecure {
                use rustls::ClientConfig;
                use std::sync::Arc;

                let mut config = ClientConfig::builder()
                    .with_root_certificates(rustls::RootCertStore::empty())
                    .with_no_client_auth();

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

        let api_key = self.api_key.clone();
        let topic = topic.map(|s| s.to_string());
        let insecure = self.insecure;

        let result = tokio::task::spawn_blocking(move || {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

            let agent = if insecure {
                use rustls::ClientConfig;
                use std::sync::Arc;

                let mut config = ClientConfig::builder()
                    .with_root_certificates(rustls::RootCertStore::empty())
                    .with_no_client_auth();

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