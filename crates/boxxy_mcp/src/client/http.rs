use anyhow::{Context, Result};
use reqwest::header::{HeaderName, HeaderValue};
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use rmcp::{RoleClient, serve_client};
use std::collections::HashMap;

pub async fn build_streamable_http_client(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<RunningService<RoleClient, ()>> {
    let mut parsed_headers = HashMap::new();
    let mut auth_header = None;

    for (k, v) in headers {
        // TODO: Check for "$KEYCHAIN:" resolution here

        // Context7 uses "Authorization: Bearer <KEY>" but rmcp Config has a dedicated `auth_header` method
        // which expects just the token.
        // Let's parse it natively so it's fully generic:
        if k.to_lowercase() == "authorization" {
            // Check if it starts with "Bearer "
            if v.to_lowercase().starts_with("bearer ") {
                auth_header = Some(v[7..].trim().to_string());
            } else {
                auth_header = Some(v.to_string());
            }
            continue;
        }

        let name = HeaderName::from_bytes(k.as_bytes())
            .with_context(|| format!("Invalid header name: {}", k))?;
        let value =
            HeaderValue::from_str(v).with_context(|| format!("Invalid header value: {}", v))?;
        parsed_headers.insert(name, value);
    }

    let mut config = StreamableHttpClientTransportConfig::with_uri(url)
        .custom_headers(parsed_headers)
        .reinit_on_expired_session(true);

    if let Some(token) = auth_header {
        config = config.auth_header(token);
    }

    let transport = StreamableHttpClientTransport::with_client(reqwest::Client::new(), config);

    let client = serve_client((), transport)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize Streamable HTTP client: {:?}", e))?;

    Ok(client)
}
