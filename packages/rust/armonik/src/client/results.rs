use futures::StreamExt;

use crate::client::client_method;
use crate::results::{upload, Raw};
use crate::rpc::services;

pub use crate::rpc::results::Client as Results;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.results.Results")]
impl<T: super::Channel> super::ServiceClient<services::Results, T> {
    /// Upload data for result with stream.
    #[armonik(rpc = "UploadResultData")]
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

    client_method!(ListResults:
        list(filters: filters<crate::results::filter::Field>, sort: plain<crate::results::Sort>, page: plain<i32>, page_size: plain<i32>)
        -> crate::results::list::Request => crate::results::list::Response);
    client_method!(GetResult:
        get(id: into<String>)
        -> crate::results::get::Request => result: crate::results::Raw);
    client_method!(GetOwnerTaskId:
        get_owner_task_id(session_id: into<String>, result_ids: iter<String>)
        -> crate::results::get_owner_task_id::Request => result_task: std::collections::HashMap<String, String>);
    client_method!(CreateResultsMetaData:
        create_metadata(session_id: into<String>, results: iter<crate::results::create_metadata::RequestItem>)
        -> crate::results::create_metadata::Request => results: Vec<crate::results::Raw>);
    client_method!(CreateResults:
        create(session_id: into<String>, results: iter<crate::results::create::RequestItem>)
        -> crate::results::create::Request => results: Vec<crate::results::Raw>);
    client_method!(DownloadResultData:
        download(session_id: into<String>, result_id: into<String>)
        -> stream crate::results::download::Request => data_chunk: bytes::Bytes);
    client_method!(DeleteResultsData:
        delete_data(session_id: into<String>, result_ids: iter<String>)
        -> crate::results::delete_data::Request => result_ids: Vec<String>);
    client_method!(ImportResultsData:
        import(session_id: into<String>, results: pairs<String, bytes::Bytes>)
        -> crate::results::import::Request => results: std::collections::HashMap<String, crate::results::Raw>);
    client_method!(GetServiceConfiguration:
        get_service_configuration()
        -> crate::results::get_service_configuration::Request => crate::results::get_service_configuration::Response);
}
