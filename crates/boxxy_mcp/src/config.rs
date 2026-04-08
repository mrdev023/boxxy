use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpTransport {
    Stdio {
        command: String,
        args: Vec<String>,
        #[serde(default)]
        env: std::collections::HashMap<String, String>, // Contract: values starting with "$KEYCHAIN:" resolve from OS Keychain at runtime.
    },
    Http {
        url: String,
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
        #[serde(default)]
        streamable: bool, // 2026 standard for high-concurrency remote tools.
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BackoffConfig {
    pub initial_ms: u64,
    pub max_ms: u64,
    pub multiplier: f32,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_ms: 1000,
            max_ms: 30000,
            multiplier: 2.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
    #[serde(default)]
    pub backoff: BackoffConfig,
}

fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    60_000
}
fn default_retries() -> u32 {
    3
}
