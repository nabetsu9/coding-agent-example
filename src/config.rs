#![allow(dead_code)] // Will be used in Task 9

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub model: ModelConfig,

    #[serde(default)]
    pub agent: AgentConfig,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_model")]
    pub default: String,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

// デフォルト値を返す関数
fn default_model() -> String {
    "claude-sonnet-4-5-20250514".to_string()
}

fn default_max_iterations() -> usize {
    10
}

// Default トレイトの実装
impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: default_model(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
        }
    }
}

impl Config {
    /// Get the codex home directory (~/.codex)
    pub fn codex_home() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".codex"))
    }

    /// Get the config file path (~/.codex/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        let codex_home = Self::codex_home()?;
        Ok(codex_home.join("config.toml"))
    }

    /// Load configuration from file (or use defaults if not found)
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            tracing::debug!("Config file not found at {:?}, using defaults", path);
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path).context("Failed to read config file")?;

        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        tracing::info!("Loaded config from {:?}", path);
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&path, content).context("Failed to write config file")?;

        tracing::info!("Saved config to {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.model.default, "claude-sonnet-4-5-20250514");
        assert_eq!(config.agent.max_iterations, 10);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // TOML文字列からデシリアライズできることを確認
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.model.default, config.model.default);
    }

    #[test]
    fn test_partial_config_parsing() {
        // 一部のフィールドが欠けていても動作することを確認
        let toml_str = r#"
[model]
default = "claude-haiku-3-5-20241022"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.model.default, "claude-haiku-3-5-20241022");
        assert_eq!(config.agent.max_iterations, 10); // デフォルト値が使われる
    }
}
