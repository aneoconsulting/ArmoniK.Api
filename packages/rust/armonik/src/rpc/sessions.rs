super::service! {
    Sessions in crate::sessions @ "armonik.api.grpc.v1.sessions.Sessions";

    rpc ListSessions(list::Request) -> list::Response;
    rpc GetSession(get::Request) -> get::Response => session;
    rpc CancelSession(cancel::Request) -> cancel::Response => session;
    rpc CreateSession(create::Request) -> create::Response => session_id;
    rpc PauseSession(pause::Request) -> pause::Response => session;
    rpc ResumeSession(resume::Request) -> resume::Response => session;
    rpc CloseSession(close::Request) -> close::Response => session;
    rpc PurgeSession(purge::Request) -> purge::Response => session;
    rpc DeleteSession(delete::Request) -> delete::Response => session;
    rpc StopSubmission(stop_submission::Request) -> stop_submission::Response => session;
}
