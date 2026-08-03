use crate::partitions::{get, list, Raw};
use crate::rpc::services;

/// The PartitionsService provides methods for interacting with partitions.
pub type Partitions<T = tonic::transport::Channel> = super::ServiceClient<services::Partitions, T>;

impl<T: super::Channel> super::ServiceClient<services::Partitions, T> {
    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = crate::partitions::filter::Field>>,
        sort: crate::partitions::Sort,
        page: i32,
        page_size: i32,
    ) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {
            filters: crate::utils::into_filters(filters),
            sort,
            page,
            page_size,
        })
        .await
    }

    pub async fn get(
        &mut self,
        partition_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(get::Request {
                partition_id: partition_id.into(),
            })
            .await?
            .partition)
    }
}

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
