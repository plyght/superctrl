use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub learning_enabled: bool,
    pub learning_db_path: PathBuf,
    pub system_prompt_path: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;

        if api_key.is_empty() {
            anyhow::bail!("ANTHROPIC_API_KEY environment variable is empty");
        }

        let learning_enabled = std::env::var("SUPERCTRL_LEARNING_ENABLED")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(true);

        let home_dir = dirs::home_dir().context("Failed to determine home directory")?;

        let superctrl_dir = home_dir.join(".superctrl");
        let learning_db_path = superctrl_dir.join("learning.db");
        let system_prompt_path = superctrl_dir.join("system_prompt.txt");

        Ok(Config {
            api_key,
            learning_enabled,
            learning_db_path,
            system_prompt_path,
        })
    }
}
