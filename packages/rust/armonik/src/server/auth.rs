use std::sync::Arc;

use crate::auth;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::auth::authentication_server as stub;

super::define_trait_methods! {
    trait AuthService {
        /// Get current user
        fn auth::current_user;
    }
}

pub trait AuthServiceExt {
    fn auth_server(self) -> stub::AuthenticationServer<Self>
    where
        Self: Sized;
}

impl<T: AuthService + Send + Sync + 'static> AuthServiceExt for T {
    fn auth_server(self) -> stub::AuthenticationServer<Self> {
        stub::AuthenticationServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::Authentication) for AuthService {
        fn get_current_user(crate::auth::current_user::Request) -> crate::auth::current_user::Response { current_user }
    }
}
