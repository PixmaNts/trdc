//! Configuration management
//!
//! Loads settings from ~/.config/trdc/config.toml with environment variable overrides.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_endpoint")]
    pub endpoint_url: String,

    #[serde(default = "default_model")]
    pub model_name: String,

    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    #[serde(default = "default_max_tokens")]
    pub max_output_tokens: usize,

    #[serde(default = "default_max_input_chars")]
    pub max_input_chars: usize,

    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint_url: default_endpoint(),
            model_name: default_model(),
            timeout_secs: default_timeout(),
            max_output_tokens: default_max_tokens(),
            max_input_chars: default_max_input_chars(),
            temperature: default_temperature(),
        }
    }
}

fn default_endpoint() -> String {
    std::env::var("TRDC_LLM_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:1234/v1/chat/completions".to_string())
}

fn default_model() -> String {
    std::env::var("TRDC_LLM_MODEL").unwrap_or_else(|_| "local-model".to_string())
}

fn default_timeout() -> u64 {
    std::env::var("TRDC_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10)
}

fn default_max_tokens() -> usize {
    std::env::var("TRDC_MAX_TOKENS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200)
}

fn default_max_input_chars() -> usize {
    4000
}

fn default_temperature() -> f32 {
    0.7
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = get_config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config from {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config from {}", path.display()))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        let path = get_config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Could not find config directory")?;
    Ok(config_dir.join("trdc").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(
            config.llm.endpoint_url,
            "http://localhost:1234/v1/chat/completions"
        );
        assert_eq!(config.llm.max_output_tokens, 200);
    }

    #[test]
    fn test_llm_config_defaults() {
        let llm = LlmConfig::default();
        assert_eq!(llm.timeout_secs, 10);
        assert_eq!(llm.max_output_tokens, 200);
        assert_eq!(llm.temperature, 0.7);
    }
}
