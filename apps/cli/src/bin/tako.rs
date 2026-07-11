#[path = "../entrypoint.rs"]
mod entrypoint;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    entrypoint::run().await
}
