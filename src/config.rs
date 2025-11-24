use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub browser_path: Option<PathBuf>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "google-patent-cli", "google-patent-cli")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        let config_path = proj_dirs.config_dir().join("config.toml");

        Self::load_from_path(&config_path)
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let proj_dirs = ProjectDirs::from("com", "google-patent-cli", "google-patent-cli")
            .context("Could not determine config directory")?;
        let config_dir = proj_dirs.config_dir();
        let config_file = config_dir.join("config.toml");

        self.save_to_path(&config_file)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(path, content).context("Failed to write config file")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.browser_path.is_none());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("google_patent_cli_test_config.toml");
        
        // Clean up before test
        if config_path.exists() {
            let _ = std::fs::remove_file(&config_path);
        }

        let mut config = Config::default();
        config.browser_path = Some(PathBuf::from("/tmp/browser"));

        config.save_to_path(&config_path).unwrap();

        let loaded_config = Config::load_from_path(&config_path).unwrap();
        assert_eq!(loaded_config.browser_path, Some(PathBuf::from("/tmp/browser")));

        // Clean up
        let _ = std::fs::remove_file(config_path);
    }
}
