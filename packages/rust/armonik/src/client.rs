use crate::{
    AuthenticationClient, PartitionsClient, ResultsClient, SubmitterClient, TasksClient,
    VersionsClient,
};

#[derive(Clone)]
pub struct Client<T> {
    channel: T,
}

impl Client<tonic::transport::Channel> {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<tonic::codegen::StdError>,
    {
        let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
        Ok(Self::new(conn))
    }
}

impl<T> Client<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self { channel }
    }

    pub fn auth(&self) -> AuthenticationClient<T> {
        AuthenticationClient::new(self.channel.clone())
    }

    pub fn partitions(&self) -> PartitionsClient<T> {
        PartitionsClient::new(self.channel.clone())
    }

    pub fn results(&self) -> ResultsClient<T> {
        ResultsClient::new(self.channel.clone())
    }

    pub fn sessions(&self) -> VersionsClient<T> {
        VersionsClient::new(self.channel.clone())
    }

    pub fn submitter(&self) -> SubmitterClient<T> {
        SubmitterClient::new(self.channel.clone())
    }

    pub fn tasks(&self) -> TasksClient<T> {
        TasksClient::new(self.channel.clone())
    }

    pub fn versions(&self) -> VersionsClient<T> {
        VersionsClient::new(self.channel.clone())
    }
}
