#[tokio::main]
async fn main() -> anyhow::Result<()> {
    google_patent_cli::cli::run().await
}
