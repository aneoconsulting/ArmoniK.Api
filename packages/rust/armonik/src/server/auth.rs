use std::sync::Arc;

use crate::api::v3;
use crate::auth;

super::define_trait_methods! {
    trait AuthService {
        /// Get current user
        fn auth::current_user;
    }
}

pub trait AuthServiceExt {
    fn auth_server(self) -> v3::auth::authentication_server::AuthenticationServer<Self>
    where
        Self: Sized;
}

impl<T: AuthService + Send + Sync + 'static> AuthServiceExt for T {
    fn auth_server(self) -> v3::auth::authentication_server::AuthenticationServer<Self> {
        v3::auth::authentication_server::AuthenticationServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::auth::authentication_server::Authentication) for AuthService {
        fn get_current_user(crate::auth::current_user::Request) -> crate::auth::current_user::Response { current_user }
    }
}
