use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn graceful_shutdown(mut child: Child) {
    // Drop stdin to signal the MCP server to exit
    drop(child.stdin.take());

    // Wait for the process to exit gracefully
    let start = Instant::now();
    let timeout = Duration::from_secs(5);

    while start.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(_)) => return, // Exited
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => break,
        }
    }

    // Fallback to kill if it doesn't exit
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_mcp_initialize() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_google-patent-cli"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn google-patent-cli mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "implementation": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });

    let request_str = init_request.to_string();
    writeln!(stdin, "{}", request_str).expect("Failed to write to stdin");

    let mut response = String::new();
    reader.read_line(&mut response).expect("Failed to read from stdout");

    // The SDK returns ServerCapabilities directly as the result
    assert!(response.contains("tools"), "Response did not contain tools: {}", response);

    // Re-attach stdin for graceful_shutdown
    child.stdin = Some(stdin);
    graceful_shutdown(child);
}

#[test]
fn test_mcp_list_tools() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_google-patent-cli"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn google-patent-cli mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // 1. Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "implementation": { "name": "test", "version": "1.0" },
            "capabilities": {}
        }
    });
    writeln!(stdin, "{}", init_request).expect("Failed to write init");
    let mut response = String::new();
    reader.read_line(&mut response).expect("Failed to read init response");
    assert!(response.contains("tools"), "Init response failed to contain tools: {}", response);

    // 2. Initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });
    writeln!(stdin, "{}", initialized_notification).expect("Failed to write initialized");

    // 3. List tools
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    writeln!(stdin, "{}", list_request).expect("Failed to write list");
    response.clear();
    reader.read_line(&mut response).expect("Failed to read list response");

    assert!(response.contains("search_patents"), "List tools response failed: {}", response);
    assert!(response.contains("fetch_patent"), "List tools response failed: {}", response);

    // Re-attach stdin for graceful_shutdown
    child.stdin = Some(stdin);
    graceful_shutdown(child);
}
