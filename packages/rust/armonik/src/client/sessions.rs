pub use crate::rpc::sessions::Client as Sessions;

use crate::client::client_method;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.sessions.Sessions")]
impl<T: super::Channel> super::ServiceClient<crate::rpc::services::Sessions, T> {
    client_method!(ListSessions:
        list(filters: filters<crate::sessions::filter::Field>, sort: plain<crate::sessions::Sort>, with_task_options: plain<bool>, page: plain<i32>, page_size: plain<i32>)
        -> crate::sessions::list::Request => crate::sessions::list::Response);

    client_method!(GetSession:
        get(session_id: into<String>)
        -> crate::sessions::get::Request => session: crate::sessions::Raw);

    client_method!(CancelSession:
        cancel(session_id: into<String>)
        -> crate::sessions::cancel::Request => session: crate::sessions::Raw);

    client_method!(CreateSession:
        create(partition_ids: iter<String>, default_task_options: plain<crate::TaskOptions>)
        -> crate::sessions::create::Request => session_id: String);

    client_method!(PauseSession:
        pause(session_id: into<String>)
        -> crate::sessions::pause::Request => session: crate::sessions::Raw);

    client_method!(ResumeSession:
        resume(session_id: into<String>)
        -> crate::sessions::resume::Request => session: crate::sessions::Raw);

    client_method!(CloseSession:
        close(session_id: into<String>)
        -> crate::sessions::close::Request => session: crate::sessions::Raw);

    client_method!(PurgeSession:
        purge(session_id: into<String>)
        -> crate::sessions::purge::Request => session: crate::sessions::Raw);

    client_method!(DeleteSession:
        delete(session_id: into<String>)
        -> crate::sessions::delete::Request => session: crate::sessions::Raw);

    client_method!(StopSubmission:
        stop_submission(session_id: into<String>, client: plain<bool>, worker: plain<bool>)
        -> crate::sessions::stop_submission::Request => session: crate::sessions::Raw);
}
