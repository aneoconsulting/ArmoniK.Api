use tracing_subscriber::{prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), eyre::Report> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    let client = armonik::Client::new().await?;

    let session = tokio::time::timeout(
        tokio::time::Duration::from_secs(1),
        client.sessions().create([""], Default::default()),
    )
    .await??;

    println!("Created session {session} using partition");

    Ok(())
}
