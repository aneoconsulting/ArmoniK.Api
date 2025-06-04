use std::sync::Arc;

use armonik::{
    partitions,
    server::{PartitionsServiceExt, RequestContext},
};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
}

impl armonik::server::PartitionsService for Service {
    async fn list(
        self: Arc<Self>,
        request: partitions::list::Request,
        _context: RequestContext,
    ) -> std::result::Result<partitions::list::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(partitions::list::Response {
                partitions: vec![partitions::Raw {
                    partition_id: String::from("rpc-list-output"),
                    ..Default::default()
                }],
                page: request.page,
                page_size: request.page_size,
                total: 1337,
            })
        })
        .await
    }

    async fn get(
        self: Arc<Self>,
        request: partitions::get::Request,
        _context: RequestContext,
    ) -> std::result::Result<partitions::get::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(partitions::get::Response {
                partition: partitions::Raw {
                    partition_id: request.partition_id,
                    parent_partition_ids: vec![String::from("rpc-get-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }
}

#[tokio::test]
async fn list() {
    let mut client =
        armonik::Client::with_channel(Service::default().partitions_server()).into_partitions();

    let response = client
        .list(
            armonik::partitions::filter::Or::default(),
            armonik::partitions::Sort::default(),
            3,
            12,
        )
        .await
        .unwrap();

    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 12);
    assert_eq!(response.total, 1337);
    assert_eq!(response.partitions[0].partition_id, "rpc-list-output");
}

#[tokio::test]
async fn get() {
    let mut client =
        armonik::Client::with_channel(Service::default().partitions_server()).into_partitions();

    let response = client.get("rpc-get-input").await.unwrap();

    assert_eq!(response.partition_id, "rpc-get-input");
    assert_eq!(response.parent_partition_ids[0], "rpc-get-output");
}
