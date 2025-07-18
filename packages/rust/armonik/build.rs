fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .use_arc_self(true)
        .build_client(cfg!(feature = "_gen-client"))
        .build_server(cfg!(feature = "_gen-server"))
        .compile_protos(
            &[
                "protos/V1/agent_common.proto",
                "protos/V1/agent_service.proto",
                "protos/V1/applications_common.proto",
                "protos/V1/applications_fields.proto",
                "protos/V1/applications_filters.proto",
                "protos/V1/applications_service.proto",
                "protos/V1/auth_common.proto",
                "protos/V1/auth_service.proto",
                "protos/V1/events_common.proto",
                "protos/V1/events_service.proto",
                "protos/V1/filters_common.proto",
                "protos/V1/objects.proto",
                "protos/V1/health_checks_common.proto",
                "protos/V1/health_checks_service.proto",
                "protos/V1/partitions_common.proto",
                "protos/V1/partitions_fields.proto",
                "protos/V1/partitions_filters.proto",
                "protos/V1/partitions_service.proto",
                "protos/V1/result_status.proto",
                "protos/V1/results_common.proto",
                "protos/V1/results_fields.proto",
                "protos/V1/results_filters.proto",
                "protos/V1/results_service.proto",
                "protos/V1/session_status.proto",
                "protos/V1/sessions_common.proto",
                "protos/V1/sessions_fields.proto",
                "protos/V1/sessions_filters.proto",
                "protos/V1/sessions_service.proto",
                "protos/V1/sort_direction.proto",
                "protos/V1/submitter_common.proto",
                "protos/V1/submitter_service.proto",
                "protos/V1/task_status.proto",
                "protos/V1/tasks_common.proto",
                "protos/V1/tasks_fields.proto",
                "protos/V1/tasks_filters.proto",
                "protos/V1/tasks_service.proto",
                "protos/V1/versions_common.proto",
                "protos/V1/versions_service.proto",
                "protos/V1/worker_common.proto",
                "protos/V1/worker_service.proto",
            ],
            &["protos/V1"],
        )?;
    Ok(())
}
