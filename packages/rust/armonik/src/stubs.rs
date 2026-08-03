//! Tonic client/server stubs, and nothing else: every message referenced by
//! their signatures is implemented natively (extern'd through the harvested
//! registry, see build.rs) and the unreferenced leftovers are pruned from the
//! stub generation, so the message-only proto packages produce no file.
//!
//! Each service's stub is re-exported as the public `stub` module of its
//! client/server module (`crate::client::sessions::stub`,
//! `crate::server::sessions::stub`); this tree only hosts the generated
//! files.

#![allow(non_snake_case)]

pub mod agent {
    tonic::include_proto!("armonik.api.grpc.v1.agent");
}
pub mod applications {
    tonic::include_proto!("armonik.api.grpc.v1.applications");
}
pub mod auth {
    tonic::include_proto!("armonik.api.grpc.v1.auth");
}
pub mod events {
    tonic::include_proto!("armonik.api.grpc.v1.events");
}
pub mod health_checks {
    tonic::include_proto!("armonik.api.grpc.v1.health_checks");
}
pub mod partitions {
    tonic::include_proto!("armonik.api.grpc.v1.partitions");
}
pub mod results {
    tonic::include_proto!("armonik.api.grpc.v1.results");
}
pub mod sessions {
    tonic::include_proto!("armonik.api.grpc.v1.sessions");
}
pub mod submitter {
    tonic::include_proto!("armonik.api.grpc.v1.submitter");
}
pub mod tasks {
    tonic::include_proto!("armonik.api.grpc.v1.tasks");
}
pub mod versions {
    tonic::include_proto!("armonik.api.grpc.v1.versions");
}
pub mod worker {
    tonic::include_proto!("armonik.api.grpc.v1.worker");
}
