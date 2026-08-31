super::service! {
    Auth in crate::auth @ "armonik.api.grpc.v1.auth.Authentication";

    rpc GetCurrentUser(current_user::Request) -> current_user::Response;
}
