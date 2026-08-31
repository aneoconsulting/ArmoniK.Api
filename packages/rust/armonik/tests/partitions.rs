use armonik::partitions;
use armonik::server::PartitionsServiceExt;

#[macro_use]
mod common;

rpc_tests! {
    client: into_partitions;
    server: PartitionsService, partitions_server;
    mock: "Partitions";

    rpc unary list {
        request: partitions::list::Request {
            filters: partitions::filter::Or::default(),
            sort: partitions::Sort::default(),
            page: 3,
            page_size: 12,
        },
        respond: |request: partitions::list::Request| partitions::list::Response {
            partitions: vec![partitions::Raw {
                partition_id: String::from("rpc-list-output"),
                ..Default::default()
            }],
            page: request.page,
            page_size: request.page_size,
            total: 1337,
        },
        convenience: list(
            partitions::filter::Or::default(),
            partitions::Sort::default(),
            3,
            12,
        ),
        check: |response| {
            assert_eq!(response.page, 3);
            assert_eq!(response.page_size, 12);
            assert_eq!(response.total, 1337);
            assert_eq!(response.partitions[0].partition_id, "rpc-list-output");
        },
    }

    rpc unary get {
        request: partitions::get::Request {
            partition_id: String::from("rpc-get-input"),
        },
        respond: |request: partitions::get::Request| partitions::get::Response {
            partition: partitions::Raw {
                partition_id: request.partition_id,
                parent_partition_ids: vec![String::from("rpc-get-output")],
                ..Default::default()
            },
        },
        convenience: get("rpc-get-input"),
        project: |response| response.partition,
        check: |partition| {
            assert_eq!(partition.partition_id, "rpc-get-input");
            assert_eq!(partition.parent_partition_ids[0], "rpc-get-output");
        },
    }
}
