use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

// ============ Help / Usage tests ============

#[test]
fn test_help() {
    cargo_bin_cmd!("google-patent-cli")
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("A CLI for searching Google Patents"));
}

#[test]
fn test_search_help() {
    cargo_bin_cmd!("google-patent-cli")
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Search query"));
}

#[test]
fn test_fetch_help() {
    cargo_bin_cmd!("google-patent-cli")
        .args(["fetch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Patent ID"));
}

// ============ Config subcommand tests ============

#[test]
fn test_config_shows_current() {
    cargo_bin_cmd!("google-patent-cli")
        .args(["config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Current configuration"));
}

// ============ Invalid usage tests ============

#[test]
fn test_no_subcommand() {
    cargo_bin_cmd!("google-patent-cli")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_search_without_query_or_assignee() {
    cargo_bin_cmd!("google-patent-cli").arg("search").assert().failure();
}

// ============ CDP specific tests ============

#[test]
fn test_head_flag_exists() {
    // Just verify the flag is accepted by help
    cargo_bin_cmd!("google-patent-cli")
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--head"));
}

#[test]
fn test_raw_flag_exists() {
    cargo_bin_cmd!("google-patent-cli")
        .args(["fetch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--raw"));
}

// ============ Execution tests (Real network/browser) ============
// Note: These tests depend on a working network and Chrome/Chromium installation.
// We use small limits and specific IDs to keep them fast.

#[test]
#[ignore = "requires network and browser"]
fn test_search_execution() {
    cargo_bin_cmd!("google-patent-cli")
        .args(["search", "--query", "machine learning", "--limit", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"id\""))
        .stdout(predicate::str::contains("\"url\""));
}

#[test]
#[ignore = "requires network and browser"]
fn test_fetch_execution() {
    cargo_bin_cmd!("google-patent-cli")
        .args(["fetch", "US9152718B2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"US9152718B2\""));
}

#[test]
#[ignore = "requires network and browser"]
fn test_fetch_raw_execution() {
    cargo_bin_cmd!("google-patent-cli")
        .args(["fetch", "US9152718B2", "--raw"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<html").or(predicate::str::contains("<HTML")));
}
