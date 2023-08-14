use tokio_stream::StreamExt;

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

    // Create result
    println!(
        "Create result: {:?}",
        client
            .results()
            .create_metadata(armonik::results::CreateResultsMetadataRequest {
                results: vec!["res1".into(), "res2".into()],
                session_id: "session-id".into(),
            })
            .await?,
    );

    // Upload result
    println!(
        "Upload result: {:?}",
        client
            .results()
            .upload(
                "session_id".into(),
                "res-1".into(),
                Box::pin(async_stream::stream! {
                    yield b"abc".to_owned();
                    yield b"def".to_owned();
                })
            )
            .await?
    );

    // Download result
    let mut response = client
        .results()
        .download(armonik::results::DownloadResultDataRequest {
            session_id: "session-id".into(),
            result_id: "res1".into(),
        })
        .await?;

    while let Some(data) = response.try_next().await? {
        println!("Data: {:?}", data);
    }

    println!("Done");

    Ok(())
}
