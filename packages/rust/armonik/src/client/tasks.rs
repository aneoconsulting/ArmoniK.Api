pub use crate::rpc::tasks::Client as Tasks;

use crate::client::client_method;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.tasks.Tasks")]
impl<T: super::Channel> super::ServiceClient<crate::rpc::services::Tasks, T> {
    client_method!(ListTasks:
        list(filters: filters<crate::tasks::filter::Field>, sort: plain<crate::tasks::Sort>, with_errors: plain<bool>, page: plain<i32>, page_size: plain<i32>)
        -> crate::tasks::list::Request => crate::tasks::list::Response);

    client_method!(ListTasksDetailed:
        list_detailed(filters: filters<crate::tasks::filter::Field>, sort: plain<crate::tasks::Sort>, with_errors: plain<bool>, page: plain<i32>, page_size: plain<i32>)
        -> crate::tasks::list_detailed::Request => crate::tasks::list_detailed::Response);

    client_method!(GetTask:
        get(task_id: into<String>)
        -> crate::tasks::get::Request => task: crate::tasks::Raw);

    client_method!(CancelTasks:
        cancel(task_ids: iter<String>)
        -> crate::tasks::cancel::Request => tasks: Vec<crate::tasks::Summary>);

    client_method!(GetResultIds:
        get_result_ids(task_ids: iter<String>)
        -> crate::tasks::get_result_ids::Request => task_results: std::collections::HashMap<String, Vec<String>>);

    client_method!(CountTasksByStatus:
        count_status(filters: filters<crate::tasks::filter::Field>)
        -> crate::tasks::count_status::Request => status: Vec<crate::StatusCount>);

    client_method!(SubmitTasks:
        submit(session_id: into<String>, task_options: plain<Option<crate::TaskOptions>>, items: iter<crate::tasks::submit::RequestItem>)
        -> crate::tasks::submit::Request => items: Vec<crate::tasks::submit::ResponseItem>);
}
