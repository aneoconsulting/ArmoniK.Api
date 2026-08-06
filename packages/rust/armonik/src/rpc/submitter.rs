super::service! {
    Submitter in crate::submitter @ "armonik.api.grpc.v1.submitter.Submitter";
    unexposed(WatchResults);
    deprecated;

    rpc GetServiceConfiguration(get_service_configuration::Request) -> get_service_configuration::Response => *;
    rpc CreateSession(create_session::Request) -> create_session::Response;
    rpc CancelSession(cancel_session::Request) -> cancel_session::Response => ();
    rpc CreateSmallTasks(create_tasks::SmallRequest) -> create_tasks::Response as create_small_tasks manual;
    rpc CreateLargeTasks(stream create_tasks::LargeRequest) -> create_tasks::Response as create_large_tasks manual;
    rpc ListTasks(list_tasks::Request) -> list_tasks::Response;
    rpc ListSessions(list_sessions::Request) -> list_sessions::Response;
    rpc CountTasks(count_tasks::Request) -> count_tasks::Response => values;
    rpc TryGetResultStream(try_get_result::Request) -> stream try_get_result::Response => *;
    rpc TryGetTaskOutput(try_get_task_output::Request) -> try_get_task_output::Response manual;
    rpc WaitForAvailability(wait_for_availability::Request) -> wait_for_availability::Response => *;
    rpc WaitForCompletion(wait_for_completion::Request) -> wait_for_completion::Response => values;
    rpc CancelTasks(cancel_tasks::Request) -> cancel_tasks::Response => ();
    rpc GetTaskStatus(task_status::Request) -> task_status::Response;
    rpc GetResultStatus(result_status::Request) -> result_status::Response;
}
