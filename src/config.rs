use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub browser_path: Option<PathBuf>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "google-patent-cli", "google-patent-cli")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        let config_path = proj_dirs.config_dir().join("config.toml");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: Self = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let proj_dirs = ProjectDirs::from("com", "google-patent-cli", "google-patent-cli")
            .context("Could not determine config directory")?;
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir).context("Failed to create config directory")?;

        let config_file = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(config_file, content).context("Failed to write config file")?;
        Ok(())
    }
}
