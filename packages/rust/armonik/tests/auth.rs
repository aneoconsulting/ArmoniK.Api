use armonik::auth;
use armonik::server::AuthServiceExt;

#[macro_use]
mod common;

rpc_tests! {
    client: into_auth;
    server: AuthService, auth_server;
    mock: "Authentication";

    rpc unary current_user {
        request: auth::current_user::Request {},
        respond: |_request| auth::current_user::Response {
            user: auth::User {
                username: String::from("rpc-current-user-output"),
                ..Default::default()
            },
        },
        convenience: current_user(),
        project: |response| response.user,
        check: |user| {
            assert_eq!(user.username, "rpc-current-user-output");
        },
    }
}
