mod agent;
mod applications;
mod auth;
mod partitions;
mod results;
mod sessions;
mod submitter;
mod tasks;
mod versions;

pub use agent::AgentClient;
pub use applications::ApplicationsClient;
pub use auth::AuthClient;
pub use partitions::PartitionsClient;
pub use results::ResultsClient;
pub use sessions::SessionsClient;
pub use submitter::SubmitterClient;
pub use tasks::TasksClient;
pub use versions::VersionsClient;

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

    pub fn agent(&self) -> AgentClient<T> {
        AgentClient::new(self.channel.clone())
    }

    pub fn applications(&self) -> ApplicationsClient<T> {
        ApplicationsClient::new(self.channel.clone())
    }

    pub fn auth(&self) -> AuthClient<T> {
        AuthClient::new(self.channel.clone())
    }

    pub fn partitions(&self) -> PartitionsClient<T> {
        PartitionsClient::new(self.channel.clone())
    }

    pub fn results(&self) -> ResultsClient<T> {
        ResultsClient::new(self.channel.clone())
    }

    pub fn sessions(&self) -> SessionsClient<T> {
        SessionsClient::new(self.channel.clone())
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

/// Perform a gRPC call from a raw request.
#[async_trait::async_trait(?Send)]
pub trait GrpcCall<Request> {
    type Response;
    type Error;

    /// Perform a gRPC call from a raw request.
    async fn call(self, request: Request) -> Result<Self::Response, Self::Error>;
}

/// Perform a gRPC call from a raw request.
#[async_trait::async_trait(?Send)]
pub trait GrpcCallStream<Request, Stream>
where
    Stream: futures::Stream<Item = Request> + Send + 'static,
{
    type Response;
    type Error;

    /// Perform a gRPC call from a raw request.
    async fn call(self, request: Stream) -> Result<Self::Response, Self::Error>;
}

#[async_trait::async_trait(?Send)]
impl<Stream, Request, T> GrpcCall<Stream> for T
where
    Stream: futures::Stream<Item = Request> + Send + 'static,
    T: GrpcCallStream<Request, Stream>,
{
    type Response = <T as GrpcCallStream<Request, Stream>>::Response;
    type Error = <T as GrpcCallStream<Request, Stream>>::Error;

    /// Perform a gRPC call from a raw request.
    async fn call(self, request: Stream) -> Result<Self::Response, Self::Error> {
        <T as GrpcCallStream<Request, Stream>>::call(self, request).await
    }
}

macro_rules! impl_call {
    (@one $Client:ident($self:ident, $request:ident: $Request:ty) -> Result<$Response:ty> $block:block) => {
        crate::client::impl_call! {
            @one $Client($self, $request: $Request) -> Result<$Response, ::tonic::Status> $block
        }
    };
    (@one $Client:ident($self:ident, $request:ident: $Request:ty) -> Result<$Response:ty, $Error:ty> $block:block) => {
        #[async_trait::async_trait(?Send)]
        impl<T> $crate::client::GrpcCall<$Request> for &'_ mut $Client<T>
        where
            T: tonic::client::GrpcService<tonic::body::BoxBody>,
            T::Error: Into<tonic::codegen::StdError>,
            T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
            <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
        {
            type Response = $Response;
            type Error = $Error;

            async fn call($self, $request: $Request) -> Result<Self::Response, Self::Error> $block
        }
    };
    ($Client:ident {$(async fn call($self:ident, $request:ident: $Request:ty) -> Result<$($Result:ty),*> $block:block)*}) => {
        $(
            crate::client::impl_call! {
                @one $Client($self, $request: $Request) -> Result<$($Result),*> $block
            }
        )*
    };
}

pub(crate) use impl_call;
