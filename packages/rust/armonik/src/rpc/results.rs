super::service! {
    Results in crate::results @ "armonik.api.grpc.v1.results.Results";
    unexposed(WatchResults);

    rpc ListResults(list::Request) -> list::Response;
    rpc GetResult(get::Request) -> get::Response;
    rpc GetOwnerTaskId(get_owner_task_id::Request) -> get_owner_task_id::Response => result_task;
    rpc CreateResultsMetaData(create_metadata::Request) -> create_metadata::Response;
    rpc CreateResults(create::Request) -> create::Response;
    rpc UploadResultData(stream upload::Request) -> upload::Response;
    rpc DownloadResultData(download::Request) -> stream download::Response;
    rpc DeleteResultsData(delete_data::Request) -> delete_data::Response => result_ids;
    rpc ImportResultsData(import::Request) -> import::Response;
    rpc GetServiceConfiguration(get_service_configuration::Request) -> get_service_configuration::Response => *;
}
