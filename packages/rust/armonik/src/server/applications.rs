use std::sync::Arc;

use crate::api::v3;
use crate::applications;

super::define_trait_methods! {
    trait ApplicationsService {
        fn applications::list;
    }
}

pub trait ApplicationsServiceExt {
    fn applications_server(self) -> v3::applications::applications_server::ApplicationsServer<Self>
    where
        Self: Sized;
}

impl<T: ApplicationsService + Send + Sync + 'static> ApplicationsServiceExt for T {
    fn applications_server(
        self,
    ) -> v3::applications::applications_server::ApplicationsServer<Self> {
        v3::applications::applications_server::ApplicationsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::applications::applications_server::Applications) for ApplicationsService {
        fn list_applications(v3::applications::ListApplicationsRequest) -> v3::applications::ListApplicationsResponse { list }
    }
}
