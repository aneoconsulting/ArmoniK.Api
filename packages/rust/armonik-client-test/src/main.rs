#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = armonik::Client::connect("http://localhost:5001").await?;

    // Get version
    let response = client.versions().list().await?;
    println!("Core: {}\nAPI: {}", response.core, response.api);

    // Get current user
    println!("{:?}", client.auth().current_user().await?);

    // List sessions
    println!("{:?}", client.sessions().list(Default::default()).await?);

    // List partitions
    let response = client
        .partitions()
        .list(armonik::partitions::PartitionListRequest {
            page: 0,
            ..Default::default()
        })
        .await?;
    println!("Partitions: {:?}", response);
    Ok(())
}
