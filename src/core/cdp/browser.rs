use crate::core::{Error, Result};
use serde_json::Value;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Chrome browser process manager
pub struct CdpBrowser {
    process: Option<Child>,
    port: u16,
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
        let chrome_path = executable_path
            .or_else(|| std::env::var("CHROME_BIN").ok().map(PathBuf::from))
            .unwrap_or_else(|| {
                #[cfg(target_os = "windows")]
                {
                    PathBuf::from("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe")
                }
                #[cfg(target_os = "macos")]
                {
                    PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
                }
                #[cfg(target_os = "linux")]
                {
                    PathBuf::from("/usr/bin/google-chrome")
                }
                #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
                {
                    PathBuf::from("chrome")
                }
            });

        // Create a temporary user data directory with a unique ID
        let unique_id = uuid::Uuid::new_v4();
        let temp_dir = std::env::temp_dir().join(format!("chrome-{}", unique_id));
        std::fs::create_dir_all(&temp_dir)?;

        let mut cmd = Command::new(&chrome_path);
        cmd.arg("--remote-debugging-port=0"); // Let OS assign a random port
        cmd.arg(format!("--user-data-dir={}", temp_dir.display()));
        cmd.arg("--password-store=basic"); // Prevent keychain prompts on macOS/Linux
        cmd.arg("--no-first-run"); // Skip first run wizards
        cmd.arg("--no-sandbox"); // Required for CI/Docker
        cmd.arg("--disable-setuid-sandbox");
        cmd.arg("--disable-dev-shm-usage"); // Prevent /dev/shm issues in CI

        if headless {
            cmd.arg("--headless=new");
        }

        for arg in args {
            cmd.arg(arg);
        }

        // Always capture stderr to read the assigned port
        // Use a temporary file for stderr to avoid buffering issues with pipes
        let stderr_file = temp_dir.join("chrome_stderr.log");
        let stderr_handle = std::fs::File::create(&stderr_file)?;

        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::from(stderr_handle));

        let process = cmd.spawn()?;

        // Read the port from the stderr file
        let port = Arc::new(StdMutex::new(None::<u16>));
        let port_clone = port.clone();
        let stderr_path = stderr_file.clone();
        let debug_flag = debug;

        // Spawn a thread to read the file and extract the port
        std::thread::spawn(move || {
            let file = match std::fs::File::open(&stderr_path) {
                Ok(f) => f,
                Err(_) => return,
            };
            let _reader = BufReader::new(file);

            // Poll the file for the port message
            for _ in 0..100 {
                // Try for 10 seconds
                // We need to re-open or seek to read new content, but simple polling works for now
                // Actually, let's just read the whole file each time since it's small
                if let Ok(content) = std::fs::read_to_string(&stderr_path) {
                    for line in content.lines() {
                        if debug_flag && line.contains("DevTools listening on") {
                            eprintln!("Chrome: {}", line);
                        }

                        if line.contains("DevTools listening on") {
                            if let Some(port_str) = line.split("127.0.0.1:").nth(1) {
                                if let Some(port_num) = port_str.split('/').next() {
                                    if let Ok(p) = port_num.parse::<u16>() {
                                        if let Ok(mut guard) = port_clone.lock() {
                                            *guard = Some(p);
                                        }
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });

        let stderr_path_for_error = stderr_file.clone();
        // Wait for the port to be discovered (up to 10 seconds)
        let discovered_port = tokio::task::spawn_blocking(move || {
            for _ in 0..100 {
                let port_val = port.lock().map_or(None, |guard| *guard);

                if let Some(p) = port_val {
                    return Ok(p);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let err_msg = match std::fs::read_to_string(&stderr_path_for_error) {
                Ok(content) => {
                    format!("Failed to discover Chrome debugging port. Chrome stderr:\n{}", content)
                }
                Err(_) => {
                    "Failed to discover Chrome debugging port. Could not read stderr.".to_string()
                }
            };
            Err(Error::Browser(err_msg))
        })
        .await
        .map_err(|e| Error::Browser(format!("Task failed: {}", e)))??;

        // Wait for Chrome to start and expose the debugging port
        // Retry get_ws_url with backoff instead of fixed sleep
        let ws_url =
            Self::get_ws_url_with_retry(discovered_port, 10, Duration::from_millis(500)).await?;

        Ok(Self { process: Some(process), port: discovered_port, ws_url })
    }

    /// Get WebSocket debugger URL from Chrome with retry logic
    async fn get_ws_url_with_retry(
        port: u16,
        max_retries: u32,
        retry_delay: Duration,
    ) -> Result<String> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match Self::get_ws_url(port).await {
                Ok(url) => return Ok(url),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        sleep(retry_delay).await;
                    }
                }
            }
        }

        Err(last_error
            .map(|e| Error::Browser(format!("Failed to get WebSocket URL after retries: {}", e)))
            .unwrap_or_else(|| {
                Error::Browser("Failed to get WebSocket URL after retries".to_string())
            }))
    }

    /// Get WebSocket debugger URL from Chrome
    async fn get_ws_url(port: u16) -> Result<String> {
        let client = reqwest::Client::new();
        let response: Value = client
            .get(format!("http://127.0.0.1:{}/json/version", port))
            .send()
            .await?
            .json()
            .await?;

        response["webSocketDebuggerUrl"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| Error::Browser("Could not find webSocketDebuggerUrl".to_string()))
    }

    /// Create a new page and return its WebSocket URL
    pub async fn new_page(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let response: Value = client
            .put(format!("http://127.0.0.1:{}/json/new", self.port))
            .send()
            .await?
            .json()
            .await?;

        response["webSocketDebuggerUrl"].as_str().map(String::from).ok_or_else(|| {
            Error::Browser("Could not find webSocketDebuggerUrl for new page".to_string())
        })
    }
}

impl Drop for CdpBrowser {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}

pub struct BrowserState {
    pub browser: Option<Arc<CdpBrowser>>,
    pub last_used: Instant,
}

#[derive(Clone)]
pub struct BrowserManager {
    browser_path: Option<PathBuf>,
    headless: bool,
    debug: bool,
    state: Arc<Mutex<BrowserState>>,
}

impl BrowserManager {
    pub fn new(browser_path: Option<PathBuf>, headless: bool, debug: bool) -> Self {
        let state = Arc::new(Mutex::new(BrowserState { browser: None, last_used: Instant::now() }));

        // Spawn the inactivity monitor task
        let state_clone = state.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await;
                let mut s = state_clone.lock().await;
                if s.browser.is_some() && s.last_used.elapsed() > Duration::from_secs(5 * 60) {
                    s.browser = None; // Drops Arc<CdpBrowser>, which triggers process kill
                }
            }
        });

        Self { browser_path, headless, debug, state }
    }

    pub async fn get_browser(&self) -> Result<Arc<CdpBrowser>> {
        let mut s = self.state.lock().await;
        s.last_used = Instant::now();

        if let Some(browser) = &s.browser {
            return Ok(Arc::clone(browser));
        }

        let mut args = vec!["--disable-blink-features=AutomationControlled"];

        if std::env::var("CI").is_ok() {
            args.push("--disable-gpu");
        }

        let browser_path = self.browser_path.clone();

        let browser =
            Arc::new(CdpBrowser::launch(browser_path, args, self.headless, self.debug).await?);
        s.browser = Some(Arc::clone(&browser));

        Ok(browser)
    }
}
