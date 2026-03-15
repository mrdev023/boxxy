use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeminiModel {
    #[serde(rename = "gemini-3.1-pro-preview")]
    Pro,
    #[serde(rename = "gemini-3.1-flash-lite-preview")]
    Flash,
}

impl fmt::Display for GeminiModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeminiModel::Pro => write!(f, "Gemini 3.1 Pro"),
            GeminiModel::Flash => write!(f, "Gemini 3.1 Flash Lite"),
        }
    }
}

impl GeminiModel {
    pub fn all() -> Vec<GeminiModel> {
        vec![GeminiModel::Pro, GeminiModel::Flash]
    }

    pub fn api_name(&self) -> &'static str {
        match self {
            GeminiModel::Pro => "gemini-3.1-pro-preview",
            GeminiModel::Flash => "gemini-3.1-flash-lite-preview",
        }
    }

    pub fn supports_thinking(&self) -> bool {
        true
    }

    pub fn available_thinking_levels(&self) -> Vec<ThinkingLevel> {
        match self {
            GeminiModel::Pro => vec![ThinkingLevel::Low, ThinkingLevel::Medium, ThinkingLevel::High],
            GeminiModel::Flash => vec![
                ThinkingLevel::Minimal,
                ThinkingLevel::Low,
                ThinkingLevel::Medium,
                ThinkingLevel::High,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingLevel {
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

impl fmt::Display for ThinkingLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThinkingLevel::Minimal => write!(f, "Minimal"),
            ThinkingLevel::Low => write!(f, "Low"),
            ThinkingLevel::Medium => write!(f, "Medium"),
            ThinkingLevel::High => write!(f, "High"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnthropicModel {
    #[serde(rename = "claude-opus-4-6")]
    ClaudeOpus,
    #[serde(rename = "claude-sonnet-4-6")]
    ClaudeSonnet,
}

impl fmt::Display for AnthropicModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnthropicModel::ClaudeOpus => write!(f, "Claude Opus 4.6"),
            AnthropicModel::ClaudeSonnet => write!(f, "Claude Sonnet 4.6"),
        }
    }
}

impl AnthropicModel {
    pub fn all() -> Vec<AnthropicModel> {
        vec![AnthropicModel::ClaudeSonnet, AnthropicModel::ClaudeOpus]
    }

    pub fn api_name(&self) -> &'static str {
        match self {
            AnthropicModel::ClaudeOpus => "claude-opus-4-6",
            AnthropicModel::ClaudeSonnet => "claude-sonnet-4-6",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelProvider {
    Gemini(GeminiModel, Option<ThinkingLevel>),
    Ollama(String),
    Anthropic(AnthropicModel),
}

impl ModelProvider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            ModelProvider::Gemini(_, _) => "Gemini",
            ModelProvider::Ollama(_) => "Ollama",
            ModelProvider::Anthropic(_) => "Anthropic",
        }
    }
}

impl Default for ModelProvider {
    fn default() -> Self {
        ModelProvider::Gemini(GeminiModel::Flash, Some(ThinkingLevel::Low))
    }
}
