use futures::StreamExt;

use crate::results::{upload, Raw};
use crate::rpc::services;

pub use crate::rpc::results::Client as Results;

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

        Ok(self.call(request).await?.result)
    }
}
