use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;

/// Chrome browser process manager
pub struct CdpBrowser {
    process: Option<Child>,
    #[allow(dead_code)]
    ws_url: String,
}

impl CdpBrowser {
    /// Launch Chrome/Chromium and get WebSocket debugger URL
    pub async fn launch(
        executable_path: Option<PathBuf>,
        args: Vec<&str>,
        headless: bool,
        debug: bool,
    ) -> Result<Self> {
        let chrome_path = executable_path.unwrap_or_else(|| {
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
        });

        // Create a temporary user data directory
        let temp_dir = std::env::temp_dir().join(format!("chrome-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir)?;

        let mut cmd = Command::new(&chrome_path);
        cmd.arg("--remote-debugging-port=9222");
        cmd.arg(format!("--user-data-dir={}", temp_dir.display()));

        if headless {
            cmd.arg("--headless");
        }

        for arg in args {
            cmd.arg(arg);
        }

        // Suppress Chrome's stdout and stderr unless in debug mode
        if !debug {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        let process = cmd.spawn()?;

        // Wait for Chrome to start and expose the debugging port
        // Retry get_ws_url with backoff instead of fixed sleep
        let ws_url = Self::get_ws_url_with_retry(10, Duration::from_millis(500)).await?;

        Ok(Self {
            process: Some(process),
            ws_url,
        })
    }

    /// Get WebSocket debugger URL from Chrome with retry logic
    async fn get_ws_url_with_retry(max_retries: u32, retry_delay: Duration) -> Result<String> {
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            match Self::get_ws_url().await {
                Ok(url) => return Ok(url),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        sleep(retry_delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("Failed to get WebSocket URL after retries")))
    }

    /// Get WebSocket debugger URL from Chrome
    async fn get_ws_url() -> Result<String> {
        let client = reqwest::Client::new();
        let response: Value = client
            .get("http://127.0.0.1:9222/json/version")
            .send()
            .await?
            .json()
            .await?;

        response["webSocketDebuggerUrl"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow!("Could not find webSocketDebuggerUrl"))
    }

    /// Create a new page and return its WebSocket URL
    pub async fn new_page(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let response: Value = client
            .put("http://127.0.0.1:9222/json/new")
            .send()
            .await?
            .json()
            .await?;

        response["webSocketDebuggerUrl"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow!("Could not find webSocketDebuggerUrl for new page"))
    }
}

impl Drop for CdpBrowser {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}
