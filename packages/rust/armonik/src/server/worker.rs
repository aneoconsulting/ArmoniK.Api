use std::sync::Arc;

use crate::worker;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::worker::worker_server as stub;

super::define_trait_methods! {
    trait WorkerService {
        fn worker::health_check;

        fn worker::process;
    }
}

pub trait WorkerServiceExt {
    fn worker_server(self) -> stub::WorkerServer<Self>
    where
        Self: Sized;
}

impl<T: WorkerService + Send + Sync + 'static> WorkerServiceExt for T {
    fn worker_server(self) -> stub::WorkerServer<Self> {
        stub::WorkerServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::Worker) for WorkerService {
        fn health_check(crate::worker::health_check::Request) -> crate::worker::health_check::Response { health_check }
        fn process(crate::worker::process::Request) -> crate::worker::process::Response { process }
    }
}
