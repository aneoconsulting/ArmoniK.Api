super::service! {
    Sessions in crate::sessions @ "armonik.api.grpc.v1.sessions.Sessions";

    rpc ListSessions(list::Request) -> list::Response;
    rpc GetSession(get::Request) -> get::Response;
    rpc CancelSession(cancel::Request) -> cancel::Response;
    rpc CreateSession(create::Request) -> create::Response;
    rpc PauseSession(pause::Request) -> pause::Response;
    rpc ResumeSession(resume::Request) -> resume::Response;
    rpc CloseSession(close::Request) -> close::Response;
    rpc PurgeSession(purge::Request) -> purge::Response;
    rpc DeleteSession(delete::Request) -> delete::Response;
    rpc StopSubmission(stop_submission::Request) -> stop_submission::Response;
}
