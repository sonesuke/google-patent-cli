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
#[command(author, version = env!("CARGO_PKG_VERSION"), about = "A CLI for searching Google Patents", long_about = include_str!("../README.md"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Args, Debug)]
struct SearchArgs {
    /// Search query
    #[arg(short, long)]
    query: Option<String>,

    /// Filter by assignee/applicant
    #[arg(long)]
    assignee: Option<String>,

    /// Filter by country (JP, US, CN)
    #[arg(long)]
    country: Option<String>,

    /// Filter by priority date after (YYYY-MM-DD)
    #[arg(short, long)]
    after: Option<String>,

    /// Filter by priority date before (YYYY-MM-DD)
    #[arg(short, long)]
    before: Option<String>,

    /// Limit the number of results
    #[arg(short, long)]
    limit: Option<usize>,

    /// Run with visible browser window (default is headless)
    #[arg(long, default_value_t = false)]
    head: bool,

    /// Output as JSON (default is JSON, but flag kept for clarity)
    #[arg(long, default_value_t = true)]
    json: bool,

    /// Debug: Connect to existing browser WS URL
    #[arg(long)]
    debug_ws_url: Option<String>,

    /// Enable debug output (shows Chrome logs)
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[derive(clap::Args, Debug)]
struct FetchArgs {
    /// Patent ID (e.g., US1234567)
    patent_id: String,

    /// Output raw HTML instead of JSON (for debugging)
    #[arg(long)]
    raw: bool,

    /// Run with visible browser window (default is headless)
    #[arg(long, default_value_t = false)]
    head: bool,

    /// Enable debug output (shows Chrome logs)
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for patents
    Search {
        #[command(flatten)]
        args: SearchArgs,
    },
    /// Fetch a specific patent by ID
    Fetch {
        #[command(flatten)]
        args: FetchArgs,
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
        Commands::Search { args } => {
            if args.query.is_none() && args.assignee.is_none() {
                anyhow::bail!("At least one of --query or --assignee must be provided.");
            }

            let config = Config::load()?;
            let searcher = PatentSearcher::new(config.browser_path, !args.head, args.debug).await?;

            let options = SearchOptions {
                query: args.query,
                assignee: args.assignee,
                country: args.country,
                patent_number: None,
                after_date: args.after,
                before_date: args.before,
                limit: args.limit,
            };

            let results = searcher.search(&options).await?;
            let json = serde_json::to_string_pretty(&results)?;
            println!("{}", json);
        }
        Commands::Fetch { args } => {
            let config = Config::load()?;
            let searcher = PatentSearcher::new(config.browser_path, !args.head, args.debug).await?;

            if args.raw {
                let html = searcher.get_raw_html(&args.patent_id).await?;
                println!("{}", html);
            } else {
                let options = SearchOptions {
                    query: None,
                    assignee: None,
                    country: None,
                    patent_number: Some(args.patent_id.clone()),
                    after_date: None,
                    before_date: None,
                    limit: None,
                };
                let mut results = searcher.search(&options).await?;
                if let Some(patent) = results.patents.pop() {
                    let json = serde_json::to_string_pretty(&patent)?;
                    println!("{}", json);
                } else {
                    eprintln!("No patent found with ID: {}", args.patent_id);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
