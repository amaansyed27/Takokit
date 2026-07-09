#[tokio::main]
async fn main() -> anyhow::Result<()> {
    takokit_cli::run().await
}
