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
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
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
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" }
        }
    });
    writeln!(stdin, "{}", init_request).expect("Failed to write init");
    let mut response = String::new();
    reader.read_line(&mut response).expect("Failed to read init response");
    assert!(response.contains("tools"), "Init response failed to contain tools: {}", response);

    // 2. Initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
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

#[test]
fn test_mcp_search_patents() {
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
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" }
        }
    });
    writeln!(stdin, "{}", init_request).expect("Failed to write init");
    let mut response = String::new();
    reader.read_line(&mut response).expect("Failed to read init response");

    // 2. Initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    writeln!(stdin, "{}", initialized_notification).expect("Failed to write initialized");

    // 3. Call search_patents
    let search_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "search_patents",
            "arguments": {
                "query": "magnetic",
                "limit": 1
            }
        }
    });
    writeln!(stdin, "{}", search_request).expect("Failed to write search call");

    response.clear();
    // This might take a while because it launches a browser
    reader.read_line(&mut response).expect("Failed to read search response");

    assert!(response.contains("result"), "Search response failed: {}", response);
    // Now returns summary with output_file and schema
    assert!(response.contains("output_file"), "Search response missing output_file: {}", response);
    assert!(response.contains("schema"), "Search response missing schema: {}", response);

    // Re-attach stdin for graceful_shutdown
    child.stdin = Some(stdin);
    graceful_shutdown(child);
}

#[test]
fn test_mcp_claims_query_patterns() {
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
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" }
        }
    });
    writeln!(stdin, "{}", init_request).unwrap();
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();

    // 2. Initialized notification
    let notif = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    writeln!(stdin, "{}", notif).unwrap();

    // 3. Fetch a patent
    let fetch_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "fetch_patent",
            "arguments": { "patent_id": "US9152718B2" }
        }
    });
    writeln!(stdin, "{}", fetch_request).unwrap();
    response.clear();
    reader.read_line(&mut response).unwrap();
    assert!(response.contains("output_file"), "Fetch failed: {}", response);

    let fetch_result: serde_json::Value = serde_json::from_str(&response).unwrap();
    let dataset = fetch_result["result"]["content"][0]["text"].as_str().unwrap();
    let dataset_json: serde_json::Value = serde_json::from_str(dataset).unwrap();
    let dataset_name = dataset_json["dataset"].as_str().unwrap().to_string();

    // 4. Query claims via label - c.text should return claim body
    let req = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "execute_cypher",
            "arguments": {
                "dataset": dataset_name,
                "query": "MATCH (c:claims) RETURN c.text LIMIT 1"
            }
        }
    });
    writeln!(stdin, "{}", req).unwrap();
    response.clear();
    reader.read_line(&mut response).unwrap();
    assert!(response.contains("c.text"), "c.text not in response: {}", response);
    assert!(!response.contains("\"c.text\": \"null\""), "c.text is null");

    // 5. Query claims via relationship - should also work
    let req = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "execute_cypher",
            "arguments": {
                "dataset": dataset_name,
                "query": "MATCH (p:Patent)-[:claims]->(c:claims) RETURN c.number, c.text LIMIT 1"
            }
        }
    });
    writeln!(stdin, "{}", req).unwrap();
    response.clear();
    reader.read_line(&mut response).unwrap();
    assert!(response.contains("c.text"), "c.text not in relationship query: {}", response);
    assert!(response.contains("c.number"), "c.number not in relationship query: {}", response);

    // Re-attach stdin for graceful_shutdown
    child.stdin = Some(stdin);
    graceful_shutdown(child);
}
