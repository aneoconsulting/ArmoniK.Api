use std::sync::Arc;

use crate::api::v3;
use crate::worker;

super::define_trait_methods! {
    trait WorkerService {
        fn worker::health_check;

        fn worker::process;
    }
}

pub trait WorkerServiceExt {
    fn worker_server(self) -> v3::worker::worker_server::WorkerServer<Self>
    where
        Self: Sized;
}

impl<T: WorkerService + Send + Sync + 'static> WorkerServiceExt for T {
    fn worker_server(self) -> v3::worker::worker_server::WorkerServer<Self> {
        v3::worker::worker_server::WorkerServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::worker::worker_server::Worker) for WorkerService {
        fn health_check(v3::Empty) -> crate::worker::health_check::Response { health_check }
        fn process(crate::worker::process::Request) -> crate::worker::process::Response { process }
    }
}
