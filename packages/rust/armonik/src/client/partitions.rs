use crate::rpc::services;

/// The PartitionsService provides methods for interacting with partitions.
pub type Partitions<T = tonic::transport::Channel> = super::ServiceClient<services::Partitions, T>;

#[cfg(test)]
#[serial_test::serial(partitions)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Partitions", "ListPartitions").await;
        let mut client = Client::new().await.unwrap().into_partitions();
        client
            .list(
                crate::partitions::filter::Or {
                    or: vec![crate::partitions::filter::And { and: vec![] }],
                },
                crate::partitions::Sort::default(),
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Partitions", "ListPartitions").await;
        assert_eq!(after - before, 1);
    }
}
