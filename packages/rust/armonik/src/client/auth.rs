pub use crate::rpc::auth::Client as Auth;

use crate::client::client_method;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.auth.Authentication")]
impl<T: super::Channel> super::ServiceClient<crate::rpc::services::Auth, T> {
    client_method!(GetCurrentUser:
        current_user()
        -> crate::auth::current_user::Request => user: crate::auth::User);
}
