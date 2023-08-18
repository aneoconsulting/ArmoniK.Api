#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = armonik::Client::connect("http://localhost:5001").await?;

    // Get version
    let response = client.versions().list().await?;
    println!("Core: {}\nAPI: {}", response.core, response.api);

    // Get current user
    println!("{:?}", client.auth().current_user().await?);

    // List partitions
    let response = client
        .partitions()
        .call(armonik::partitions::list::Request {
            page: 0,
            ..Default::default()
        })
        .await?;
    println!("Partitions: {:?}", response);
    let partition = response.partitions[0].id.clone();

    // Create session
    let session = client
        .sessions()
        .create(vec![partition.clone()], Default::default())
        .await?;

    println!("Created session {session} using partition {partition}");

    // Create result
    let mut results = client
        .results()
        .create_metadata(
            session.clone(),
            ["input".into(), "output".into()].into_iter().collect(),
        )
        .await?;

    let input = results.remove("input").unwrap();
    let output = results.remove("output").unwrap();

    // Upload payload
    client
        .results()
        .upload(
            session.clone(),
            input.result_id.clone(),
            Box::pin(async_stream::stream! {
                yield b"payload".to_vec();
            }),
        )
        .await?;

    // Submit task
    client
        .tasks()
        .submit(
            session.clone(),
            None,
            vec![armonik::tasks::submit::RequestItem {
                expected_output_keys: vec![output.result_id.clone()],
                data_dependencies: vec![],
                payload_id: input.result_id.clone(),
                task_options: None,
            }],
        )
        .await?;

    println!("Done");

    Ok(())
}
