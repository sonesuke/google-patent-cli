use crate::core::{Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub browser_path: Option<PathBuf>,
    #[serde(default)]
    pub chrome_args: Vec<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "google-patent-cli", "google-patent-cli")
            .ok_or_else(|| Error::Config("Could not determine config directory".to_string()))?;
        let config_path = proj_dirs.config_dir().join("config.toml");

        Self::load_from_path(&config_path)
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let proj_dirs = ProjectDirs::from("com", "google-patent-cli", "google-patent-cli")
            .ok_or_else(|| Error::Config("Could not determine config directory".to_string()))?;
        let config_dir = proj_dirs.config_dir();
        let config_file = config_dir.join("config.toml");

        self.save_to_path(&config_file)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::Config(format!("Failed to create config directory: {}", e)))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, content)
            .map_err(|e| Error::Config(format!("Failed to write config file: {}", e)))?;
        Ok(())
    }

    /// Resolve browser path and chrome args with priority:
    /// 1. config.toml values (highest priority)
    /// 2. CI environment (CI=1)
    /// 3. Auto-detection (fallback)
    pub fn resolve(&self) -> (Option<PathBuf>, Vec<String>) {
        // Priority 1: Use config.toml values if explicitly set
        if self.browser_path.is_some() || !self.chrome_args.is_empty() {
            return (self.browser_path.clone(), self.chrome_args.clone());
        }

        // Priority 2: CI environment
        if (env::var("CI").is_ok() || env::var("GITHUB_ACTIONS").is_ok())
            && let Some(path) = detect_chrome_path()
        {
            // CI typically runs in containers/VMs, so add --no-sandbox
            return (Some(path), vec!["--no-sandbox".to_string()]);
        }

        // Priority 3: Auto-detection
        let path = detect_chrome_path();
        let mut args = vec![];

        // Add --no-sandbox for containerized environments
        if is_running_in_container() {
            args.push("--no-sandbox".to_string());
        }

        (path, args)
    }
}

/// Detect Chrome/Chromium path from common locations
fn detect_chrome_path() -> Option<PathBuf> {
    let candidates = [
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium-browser",
        "/usr/bin/chromium",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Detect if running in a containerized environment (Docker/Kubernetes)
fn is_running_in_container() -> bool {
    // Check for .dockerenv file (Docker)
    if PathBuf::from("/.dockerenv").exists() {
        return true;
    }

    // Check /proc/1/cgroup for container indicators
    if let Ok(content) = fs::read_to_string("/proc/1/cgroup")
        && (content.contains("docker")
            || content.contains("kubepods")
            || content.contains("containerd"))
    {
        return true;
    }

    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::field_reassign_with_default)]
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

    #[test]
    fn test_resolve_with_config_values() {
        // When browser_path is set, it should be used
        let config = Config {
            browser_path: Some(PathBuf::from("/custom/chrome")),
            chrome_args: vec!["--custom-arg".to_string()],
        };

        let (path, args) = config.resolve();
        assert_eq!(path, Some(PathBuf::from("/custom/chrome")));
        assert_eq!(args, vec!["--custom-arg".to_string()]);
    }

    #[test]
    fn test_resolve_auto_detect() {
        // When no config is set, should auto-detect (or return None if not found)
        let config = Config::default();

        let (_path, args) = config.resolve();
        // We can't assert the exact path since it depends on the system
        // But we can verify that args contains --no-sandbox if in container
        if is_running_in_container() {
            assert!(args.contains(&"--no-sandbox".to_string()));
        }
    }
}
