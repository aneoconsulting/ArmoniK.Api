use crate::{
    api::v3,
    objects::partitions::{get, list, Raw},
};

use super::GrpcCall;

#[derive(Clone)]
pub struct PartitionsClient<T> {
    inner: v3::partitions::partitions_client::PartitionsClient<T>,
}

impl<T> PartitionsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::partitions::partitions_client::PartitionsClient::new(channel),
        }
    }

    pub async fn list(&mut self, request: list::Request) -> Result<list::Response, tonic::Status> {
        self.call(request).await
    }

    pub async fn get(&mut self, partition_id: impl Into<String>) -> Result<Raw, tonic::Status> {
        Ok(self
            .call(get::Request {
                id: partition_id.into(),
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
    PartitionsClient {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            Ok(self
                .inner
                .list_partitions(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: get::Request) -> Result<get::Response> {
            Ok(self
                .inner
                .get_partition(request)
                .await?
                .into_inner()
                .into())
        }
    }
}
