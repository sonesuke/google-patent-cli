pub mod cli;
pub mod core;
pub mod mcp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run().await
}
