super::service! {
    Tasks in crate::tasks @ "armonik.api.grpc.v1.tasks.Tasks";

    rpc ListTasks(list::Request) -> list::Response;
    rpc ListTasksDetailed(list_detailed::Request) -> list_detailed::Response;
    rpc GetTask(get::Request) -> get::Response;
    rpc CancelTasks(cancel::Request) -> cancel::Response;
    rpc GetResultIds(get_result_ids::Request) -> get_result_ids::Response;
    rpc CountTasksByStatus(count_status::Request) -> count_status::Response;
    rpc SubmitTasks(submit::Request) -> submit::Response;
}
