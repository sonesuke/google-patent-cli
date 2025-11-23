//! Google Patent CLI
//!
//! A command-line tool for searching Google Patents.
//!
//! # Usage
//!
//! ```bash
//! google-patent-cli search --query "machine learning"
//! ```

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::all, clippy::nursery)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cdp;
mod config;
mod models;
mod patent_search;

use config::Config;
use models::SearchOptions;
use patent_search::PatentSearcher;

#[derive(Parser)]
#[command(name = "google-patent-cli")]
#[command(author, version = env!("CARGO_PKG_VERSION"), about = "A CLI for searching Google Patents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for patents
    Search {
        /// Search query
        #[arg(short, long)]
        query: Option<String>,

        /// Patent number (e.g., US1234567)
        #[arg(short, long)]
        patent: Option<String>,

        /// Filter by priority date after (YYYY-MM-DD)
        #[arg(short, long)]
        after: Option<String>,

        /// Filter by priority date before (YYYY-MM-DD)
        #[arg(short, long)]
        before: Option<String>,

        /// Run with visible browser window (default is headless)
        #[arg(long, default_value_t = false)]
        head: bool,

        /// Output as JSON (default is JSON, but flag kept for clarity)
        #[arg(long, default_value_t = true)]
        json: bool,

        /// Output raw HTML instead of JSON (for debugging)
        #[arg(long)]
        raw: bool,

        /// Debug: Connect to existing browser WS URL
        #[arg(long)]
        debug_ws_url: Option<String>,

        /// Enable debug output (shows Chrome logs)
        #[arg(long, default_value_t = false)]
        debug: bool,
    },
    /// Configure the CLI
    Config {
        /// Set the path to the browser executable
        #[arg(long)]
        set_browser: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search {
            query,
            patent,
            after,
            before,
            head,
            debug,
            raw,
            // json and debug_ws_url are not used in this branch anymore
            json: _,
            debug_ws_url: _,
        } => {
            let searcher = PatentSearcher::new(!head, debug).await?;

            if raw {
                // Output raw HTML for debugging
                if let Some(patent_id) = patent {
                    let html = searcher.get_raw_html(&patent_id).await?;
                    println!("{}", html);
                } else {
                    eprintln!("Error: --raw flag requires --patent <ID>");
                    std::process::exit(1);
                }
            } else {
                // Normal JSON output
                let options = SearchOptions {
                    query,
                    patent_number: patent,
                    after_date: after,
                    before_date: before,
                };

                let results = searcher.search(&options).await?;
                let json = serde_json::to_string_pretty(&results)?;
                println!("{}", json);
            }
        }
        Commands::Config { set_browser } => {
            let mut config = Config::load()?;
            if let Some(path) = set_browser {
                config.browser_path = Some(path);
                config.save()?;
                println!("Browser path updated.");
            } else {
                println!("Current configuration:");
                println!("{:#?}", config);
            }
        }
    }

    Ok(())
}
