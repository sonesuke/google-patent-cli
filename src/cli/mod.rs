use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::core::config::Config;
use crate::core::models::SearchOptions;
use crate::core::patent_search::{PatentSearch, PatentSearcher};
use crate::mcp;

#[derive(Parser)]
#[command(name = "google-patent-cli")]
#[command(author, version = env!("CARGO_PKG_VERSION"), about = "A CLI for searching Google Patents", long_about = include_str!("../../README.md"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Args, Debug)]
pub struct SearchArgs {
    /// Search query
    #[arg(short, long)]
    pub query: Option<String>,

    /// Filter by assignee/applicant
    #[arg(long, num_args = 1..)]
    pub assignee: Option<Vec<String>>,

    /// Filter by country (JP, US, CN)
    #[arg(long)]
    pub country: Option<String>,

    /// Filter by priority date after (YYYY-MM-DD)
    #[arg(short, long)]
    pub after: Option<String>,

    /// Filter by priority date before (YYYY-MM-DD)
    #[arg(short, long)]
    pub before: Option<String>,

    /// Limit the number of results
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Run with visible browser window (default is headless)
    #[arg(long, default_value_t = false)]
    pub head: bool,

    /// Output as JSON (default is JSON, but flag kept for clarity)
    #[arg(long, default_value_t = true)]
    pub json: bool,

    /// Debug: Connect to existing browser WS URL
    #[arg(long)]
    pub debug_ws_url: Option<String>,

    /// Enable debug output (shows Chrome logs)
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// Enable verbose output (shows detailed progress)
    #[arg(long, default_value_t = false)]
    pub verbose: bool,

    /// Language/locale for patent pages (e.g., ja, en, zh)
    #[arg(long)]
    pub language: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct FetchArgs {
    /// Patent ID (e.g., US1234567)
    pub patent_id: String,

    /// Output raw HTML instead of JSON (for debugging)
    #[arg(long)]
    pub raw: bool,

    /// Run with visible browser window (default is headless)
    #[arg(long, default_value_t = false)]
    pub head: bool,

    /// Enable debug output (shows Chrome logs)
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// Enable verbose output (shows detailed progress)
    #[arg(long, default_value_t = false)]
    pub verbose: bool,

    /// Language/locale for patent pages (e.g., ja, en, zh)
    #[arg(long)]
    pub language: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
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
    /// Start MCP server
    Mcp,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    run_app(cli).await
}

pub async fn run_app(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Mcp => {
            mcp::run().await?;
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
        Commands::Search { args } => {
            if args.query.is_none() && args.assignee.is_none() {
                anyhow::bail!("At least one of --query or --assignee must be provided.");
            }

            let config = Config::load()?;
            let (browser_path, chrome_args) = config.resolve();
            let searcher = PatentSearcher::new(
                browser_path,
                !args.head,
                args.debug,
                args.verbose,
                chrome_args,
            )
            .await?;

            let options = SearchOptions {
                query: args.query,
                assignee: args.assignee,
                country: args.country,
                patent_number: None,
                after_date: args.after,
                before_date: args.before,
                limit: args.limit,
                language: args.language,
            };

            let results = searcher.search(&options).await?;
            let json = serde_json::to_string_pretty(&results)?;
            println!("{}", json);
        }
        Commands::Fetch { args } => {
            let config = Config::load()?;
            let (browser_path, chrome_args) = config.resolve();
            let searcher = PatentSearcher::new(
                browser_path,
                !args.head,
                args.debug,
                args.verbose,
                chrome_args,
            )
            .await?;

            if args.raw {
                let html = searcher.get_raw_html(&args.patent_id, args.language.as_deref()).await?;
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
                    language: args.language,
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(["google-patent-cli", "search", "--query", "test"]);
        assert!(cli.is_ok());

        let cli = Cli::try_parse_from(["google-patent-cli", "fetch", "US123"]);
        assert!(cli.is_ok());

        let cli = Cli::try_parse_from([
            "google-patent-cli",
            "config",
            "--set-browser",
            "/path/to/browser",
        ]);
        assert!(cli.is_ok());
    }

    #[tokio::test]
    async fn test_run_app_config_list() {
        // This will print to stdout, but we can check if it returns Ok
        let cli = Cli::try_parse_from(["google-patent-cli", "config"])
            .expect("Cli parsing success in test");
        let res = run_app(cli).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_run_app_search_no_args() {
        let cli = Cli::try_parse_from(["google-patent-cli", "search"])
            .expect("Cli parsing success in test");
        let res = run_app(cli).await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "At least one of --query or --assignee must be provided."
        );
    }

    #[tokio::test]
    async fn test_run_app_config_set() {
        let temp_dir = tempfile::tempdir().expect("Cli parsing success in test");
        let browser_path = temp_dir.path().join("browser");
        let cli = Cli::try_parse_from([
            "google-patent-cli",
            "config",
            "--set-browser",
            browser_path.to_str().expect("Valid UTF-8 path"),
        ])
        .expect("Cli parsing success in test");

        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());
        std::env::set_var("APPDATA", temp_dir.path());
        std::env::set_var("USERPROFILE", temp_dir.path());

        let res = run_app(cli).await;
        assert!(res.is_ok());

        let config = Config::load().expect("Failed to load config");
        assert_eq!(config.browser_path, Some(browser_path));
    }
}
