use snafu::ResultExt;

use crate::api::v3;
use crate::applications::{filter, list, Sort};
use crate::utils::IntoCollection;

use super::GrpcCall;

#[derive(Clone)]
pub struct ApplicationsClient<T> {
    inner: v3::applications::applications_client::ApplicationsClient<T>,
}

impl<T> ApplicationsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::applications::applications_client::ApplicationsClient::new(channel),
        }
    }

    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = filter::Field>>,
        sort: Sort,
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
    ApplicationsClient {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            Ok(self
                .inner
                .list_applications(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }
    }
}

#[cfg(test)]
#[serial_test::serial(applications)]
mod tests {
    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Applications", "ListApplications").await;
        let mut client = Client::new().await.unwrap().applications();
        client
            .list(
                crate::applications::filter::Or {
                    or: vec![crate::applications::filter::And { and: vec![] }],
                },
                crate::applications::Sort::default(),
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Applications", "ListApplications").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn list_call() {
        let before = Client::get_nb_request("Applications", "ListApplications").await;
        let mut client = Client::new().await.unwrap().applications();
        client
            .call(crate::applications::list::Request {
                page_size: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Applications", "ListApplications").await;
        assert_eq!(after - before, 1);
    }
}
