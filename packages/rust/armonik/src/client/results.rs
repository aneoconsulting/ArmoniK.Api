use futures::StreamExt;

use crate::results::{upload, Raw};
use crate::rpc::services;

/// The ResultsService provides methods for interacting with results.
pub type Results<T = tonic::transport::Channel> = super::ServiceClient<services::Results, T>;

impl<T: super::Channel> super::ServiceClient<services::Results, T> {
    /// Upload data for result with stream.
    pub async fn upload<S>(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
        data: S,
    ) -> Result<Raw, super::RequestError>
    where
        S: futures::Stream + Send + 'static,
        <S as futures::Stream>::Item: Into<bytes::Bytes>,
    {
        let request = futures::stream::iter([upload::Request::Identifier {
            session_id: session_id.into(),
            result_id: result_id.into(),
        }]);
        let request = request.chain(data.map(|chunk| upload::Request::DataChunk(chunk.into())));

        Ok(self.call_streaming(request).await?.result)
    }

}

#[cfg(test)]
#[serial_test::serial(results)]
mod tests {
    use crate::Client;
    use futures::TryStreamExt;


    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Results", "ListResults").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .list(
                crate::results::filter::Or {
                    or: vec![crate::results::filter::And { and: vec![] }],
                },
                crate::results::Sort::default(),
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "ListResults").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn upload() {
        let before = Client::get_nb_request("Results", "UploadResultData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .upload(
                "session-id",
                "result-id",
                futures::stream::iter([bytes::Bytes::new()]),
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "UploadResultData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn download() {
        let before = Client::get_nb_request("Results", "DownloadResultData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .download("session-id", "result-id")
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "DownloadResultData").await;
        assert_eq!(after - before, 1);
    }
}
