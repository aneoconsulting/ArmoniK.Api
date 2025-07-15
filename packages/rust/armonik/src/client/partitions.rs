use snafu::ResultExt;

use crate::api::v3;
use crate::partitions::{get, list, Raw};
use crate::utils::IntoCollection;

use super::GrpcCall;

#[derive(Clone)]
pub struct Partitions<T> {
    inner: v3::partitions::partitions_client::PartitionsClient<T>,
}

impl<T> Partitions<T>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::partitions::partitions_client::PartitionsClient::new(channel),
        }
    }

    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = crate::partitions::filter::Field>>,
        sort: crate::partitions::Sort,
        page: i32,
        page_size: i32,
    ) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {
            filters: filters
                .into_iter()
                .map(IntoCollection::into_collect)
                .collect(),
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

    /// Perform a gRPC call from a raw request.
    pub async fn call<Request>(
        &mut self,
        request: Request,
    ) -> Result<<&mut Self as GrpcCall<Request>>::Response, <&mut Self as GrpcCall<Request>>::Error>
    where
        for<'a> &'a mut Self: GrpcCall<Request>,
    {
        <&mut Self as GrpcCall<Request>>::call(self, request).await
    }
}

super::impl_call! {
    Partitions {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .list_partitions(request),
                tracing::debug_span!("Partitions::list")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: get::Request) -> Result<get::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .get_partition(request),
                tracing::debug_span!("Partitions::get")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }
    }
}

#[cfg(test)]
#[serial_test::serial(partitions)]
mod tests {
    use crate::Client;

    // Named methods

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

    #[tokio::test]
    async fn get() {
        let before = Client::get_nb_request("Partitions", "GetPartition").await;
        let mut client = Client::new().await.unwrap().into_partitions();
        client.get("part1").await.unwrap();
        let after = Client::get_nb_request("Partitions", "GetPartition").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn list_call() {
        let before = Client::get_nb_request("Partitions", "ListPartitions").await;
        let mut client = Client::new().await.unwrap().into_partitions();
        client
            .call(crate::partitions::list::Request {
                page_size: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Partitions", "ListPartitions").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_call() {
        let before = Client::get_nb_request("Partitions", "GetPartition").await;
        let mut client = Client::new().await.unwrap().into_partitions();
        client
            .call(crate::partitions::get::Request {
                partition_id: String::from("part1"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Partitions", "GetPartition").await;
        assert_eq!(after - before, 1);
    }
}
