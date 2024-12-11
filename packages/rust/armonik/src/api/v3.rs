#![allow(non_snake_case)]

tonic::include_proto!("armonik.api.grpc.v1");

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
pub mod result_status {
    tonic::include_proto!("armonik.api.grpc.v1.result_status");
}
pub mod results {
    tonic::include_proto!("armonik.api.grpc.v1.results");
}
pub mod session_status {
    tonic::include_proto!("armonik.api.grpc.v1.session_status");
}
pub mod sessions {
    tonic::include_proto!("armonik.api.grpc.v1.sessions");
}
pub mod sort_direction {
    tonic::include_proto!("armonik.api.grpc.v1.sort_direction");
}
pub mod submitter {
    tonic::include_proto!("armonik.api.grpc.v1.submitter");
}
pub mod task_status {
    tonic::include_proto!("armonik.api.grpc.v1.task_status");
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
