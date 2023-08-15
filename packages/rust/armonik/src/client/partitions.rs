use crate::{
    api::v3,
    objects::partitions::{PartitionListRequest, PartitionListResponse, PartitionRaw},
};

#[derive(Clone)]
pub struct PartitionsClient<T> {
    inner: v3::partitions::partitions_client::PartitionsClient<T>,
}

impl<T> PartitionsClient<T>
where
    T: Clone,
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

    pub async fn list(
        &mut self,
        request: PartitionListRequest,
    ) -> Result<PartitionListResponse, tonic::Status> {
        Ok(self
            .inner
            .list_partitions(request)
            .await?
            .into_inner()
            .into())
    }

    pub async fn get(&mut self, partition_id: String) -> Result<PartitionRaw, tonic::Status> {
        Ok(self
            .inner
            .get_partition(v3::partitions::GetPartitionRequest { id: partition_id })
            .await?
            .into_inner()
            .partition
            .into())
    }
}
