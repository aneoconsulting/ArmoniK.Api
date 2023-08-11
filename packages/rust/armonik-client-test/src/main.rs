#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = armonik::Client::connect("http://localhost:5001").await?;

    // Get version
    let response = client
        .versions()
        .list_versions(armonik::versions::ListVersionsRequest {})
        .await?
        .into_inner();
    println!("Core: {}\nAPI: {}", response.core, response.api);

    // Get configuration
    let response = client
        .submitter()
        .get_service_configuration(armonik::Empty {})
        .await?
        .into_inner();
    println!("Configuration: {:?}", response);

    // List partitions
    let response = client
        .partitions()
        .list_partitions(armonik::partitions::ListPartitionsRequest {
            page: 0,
            page_size: 100,
            filters: Some(armonik::partitions::Filters {
                or: vec![armonik::partitions::FiltersAnd {
                    and: vec![armonik::partitions::FilterField {
                        field: Some(armonik::partitions::PartitionField {
                            field: Some(
                                armonik::partitions::partition_field::Field::PartitionRawField(
                                    armonik::partitions::PartitionRawField {
                                        field: armonik::partitions::PartitionRawEnumField::Id
                                            as i32,
                                    },
                                ),
                            ),
                        }),
                        value_condition: Some(
                            armonik::partitions::filter_field::ValueCondition::FilterString(
                                armonik::FilterString {
                                    value: String::default(),
                                    operator: armonik::FilterStringOperator::Contains as i32,
                                },
                            ),
                        ),
                    }],
                }],
            }),
            sort: Some(armonik::partitions::list_partitions_request::Sort {
                field: Some(armonik::partitions::PartitionField {
                    field: Some(
                        armonik::partitions::partition_field::Field::PartitionRawField(
                            armonik::partitions::PartitionRawField {
                                field: armonik::partitions::PartitionRawEnumField::Id as i32,
                            },
                        ),
                    ),
                }),
                direction: armonik::sort_direction::SortDirection::Asc as i32,
            }),
        })
        .await?
        .into_inner();
    println!("Partitions: {:?}", response);
    Ok(())
}
