super::service! {
    Agent in crate::agent @ "armonik.api.grpc.v1.agent.Agent";

    rpc CreateTask(stream create_tasks::Request) -> create_tasks::Response;
    rpc CreateResultsMetaData(create_results_metadata::Request) -> create_results_metadata::Response => results;
    rpc CreateResults(create_results::Request) -> create_results::Response => results;
    rpc NotifyResultData(notify_result_data::Request) -> notify_result_data::Response => result_ids;
    rpc SubmitTasks(submit_tasks::Request) -> submit_tasks::Response => items;
    rpc GetResourceData(get_resource_data::Request) -> get_resource_data::Response;
    rpc GetCommonData(get_common_data::Request) -> get_common_data::Response;
    rpc GetDirectData(get_direct_data::Request) -> get_direct_data::Response;
}
