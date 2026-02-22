use crate::core::{Error, Result};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

use super::connection::CdpConnection;

/// CDP Page for browser automation
pub struct CdpPage {
    connection: CdpConnection,
}

impl CdpPage {
    /// Create a new page with the given connection
    pub async fn new(ws_url: &str) -> Result<Self> {
        let connection = CdpConnection::connect(ws_url).await?;

        // Enable necessary domains
        if let Err(e) = connection.send_command("Page.enable", json!({})).await {
            return Err(Error::Browser(format!("Failed to enable Page domain: {}", e)));
        }
        if let Err(e) = connection.send_command("Runtime.enable", json!({})).await {
            return Err(Error::Browser(format!("Failed to enable Runtime domain: {}", e)));
        }

        Ok(Self { connection })
    }

    /// Navigate to a URL
    pub async fn goto(&self, url: &str) -> Result<()> {
        if let Err(e) = self.connection.send_command("Page.navigate", json!({ "url": url })).await {
            return Err(Error::Browser(format!("Failed to navigate to URL '{}': {}", url, e)));
        }

        Ok(())
    }

    /// Wait for an element to appear on the page
    pub async fn wait_for_element(&self, selector: &str, timeout_secs: u64) -> Result<bool> {
        let start = std::time::Instant::now();

        while start.elapsed().as_secs() < timeout_secs {
            let script = format!("!!document.querySelector(\"{}\")", selector.replace('"', "\\\""));

            let result = self.evaluate(&script).await?;
            if result.as_bool().unwrap_or(false) {
                return Ok(true);
            }

            sleep(Duration::from_millis(500)).await;
        }

        Ok(false)
    }

    /// Get full HTML content for debugging
    pub async fn get_html(&self) -> Result<String> {
        let script = "document.documentElement.outerHTML";
        let result = self.evaluate(script).await?;
        result.as_str().map(String::from).ok_or_else(|| {
            Error::Browser("Failed to get HTML: JavaScript result was not a string".to_string())
        })
    }

    /// Evaluate JavaScript and return the result
    pub async fn evaluate(&self, script: &str) -> Result<Value> {
        let result = self
            .connection
            .send_command(
                "Runtime.evaluate",
                json!({
                    "expression": script,
                    "returnByValue": true,
                    "awaitPromise": true
                }),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let exception_text = exception["exception"]["description"]
                .as_str()
                .or_else(|| exception["text"].as_str())
                .unwrap_or("unknown error");

            let column_number = exception["columnNumber"].as_i64().unwrap_or(-1);
            let line_number = exception["lineNumber"].as_i64().unwrap_or(-1);

            return Err(Error::Browser(format!(
                "JavaScript execution error at line {}, column {}: {}",
                line_number, column_number, exception_text
            )));
        }

        Ok(result["result"]["value"].clone())
    }

    /// Close the page/tab
    pub async fn close(&self) -> Result<()> {
        self.connection
            .send_command("Page.close", json!({}))
            .await
            .map_err(|e| Error::Browser(format!("Failed to close page: {}", e)))?;
        Ok(())
    }
}
