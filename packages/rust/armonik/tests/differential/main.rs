//! Differential harness: randomized `DynamicMessage`s generated from the
//! real protobuf descriptors are round-tripped through the armonik types
//! (decode + re-encode) and compared semantically; a coverage test ratchets
//! the whole descriptor pool into the registry as the migration proceeds.
//!
//! Every failure prints the seed needed to replay the exact case.

mod arbitrary;
mod compare;
mod registry;
mod rng;

use armonik::reexports::prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};

static DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

const ITERATIONS: u64 = 64;
const RECURSION_DEPTH: u32 = 3;

fn pool() -> DescriptorPool {
    DescriptorPool::decode(DESCRIPTOR).expect("embedded descriptor set decodes")
}

/// Compact `name: value` dump of the set fields (recursive), for failure
/// messages — the `Debug` impl of `DynamicMessage` prints whole descriptors.
fn debug_fields(message: &DynamicMessage) -> String {
    use prost_reflect::ReflectMessage;
    use std::fmt::Write;

    let mut out = String::from("{ ");
    for field in message.descriptor().fields() {
        if !message.has_field(&field) {
            continue;
        }
        let value = message.get_field(&field);
        let _ = write!(out, "{}: {}, ", field.name(), debug_value(value.as_ref()));
    }
    out.push('}');
    out
}

fn debug_value(value: &prost_reflect::Value) -> String {
    match value {
        prost_reflect::Value::Message(inner) => debug_fields(inner),
        prost_reflect::Value::List(items) => {
            let items: Vec<String> = items.iter().map(debug_value).collect();
            format!("[{}]", items.join(", "))
        }
        other => format!("{other:?}"),
    }
}

#[test]
fn registered_types_roundtrip() {
    let pool = pool();
    for entry in registry::entries() {
        let desc = pool
            .get_message_by_name(entry.proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", entry.proto));
        for iteration in 0..ITERATIONS {
            let seed = rng::seed(entry.proto, iteration);
            let mut rng = rng::SplitMix64::new(seed);
            let mut original = arbitrary::message(&desc, &mut rng, RECURSION_DEPTH);
            let bytes = original.encode_to_vec();

            let reencoded = (entry.roundtrip)(&bytes).unwrap_or_else(|err| {
                panic!(
                    "armonik type failed to decode `{}` (seed {seed:#018x}): {err}\n\
                     original: {original:#?}",
                    entry.proto
                )
            });
            let mut back = DynamicMessage::decode(desc.clone(), reencoded.as_slice())
                .unwrap_or_else(|err| {
                    panic!(
                        "re-encoded bytes of `{}` do not decode (seed {seed:#018x}): {err}\n\
                         original: {original:#?}",
                        entry.proto
                    )
                });

            registry::normalize(&mut original);
            registry::normalize(&mut back);

            assert!(
                compare::messages(&original, &back),
                "semantic mismatch for `{}` (seed {seed:#018x})\n\
                 original:   {}\n\
                 round-trip: {}",
                entry.proto,
                debug_fields(&original),
                debug_fields(&back),
            );
        }
    }
}

/// Messages that never get their own Rust type: they are flattened into
/// their parent's representation (wrappers, pair entries) and are covered
/// through the parent's round-trips.
const PERMANENT_UNMAPPED: &[&str] = &[
    // Single-enum-field wrappers flattened into `TaskOptionField`; their wire
    // form is unit-tested in `objects/task_options.rs` and exercised through
    // every message embedding them.
    "armonik.api.grpc.v1.sessions.TaskOptionField",
    "armonik.api.grpc.v1.tasks.TaskOptionField",
    // Marker payload of `Output::Ok`; carries no data.
    "armonik.api.grpc.v1.Empty",
    // Inlined into the `Output::Error` struct variant.
    "armonik.api.grpc.v1.Output.Error",
    // Inlined into the `agent::create_tasks::Status::TaskInfo` variant.
    "armonik.api.grpc.v1.agent.CreateTaskReply.TaskInfo",
    // Flattened into `agent::create_tasks::Response::Status.statuses`.
    "armonik.api.grpc.v1.agent.CreateTaskReply.CreationStatusList",
    // Enum wrapper chain flattened into `applications::Field`.
    "armonik.api.grpc.v1.applications.ApplicationField",
    "armonik.api.grpc.v1.applications.ApplicationRawField",
];

/// Messages not yet migrated to a direct wire implementation. This list
/// only shrinks: annotating a type moves it to the registry, and the test
/// fails on stale entries. It must be empty by the end of the migration.
const TEMP_UNMAPPED: &[&str] = &[
    "armonik.api.grpc.v1.agent.CreateResultsMetaDataRequest",
    "armonik.api.grpc.v1.agent.CreateResultsMetaDataRequest.ResultCreate",
    "armonik.api.grpc.v1.agent.CreateResultsMetaDataResponse",
    "armonik.api.grpc.v1.agent.CreateResultsRequest",
    "armonik.api.grpc.v1.agent.CreateResultsRequest.ResultCreate",
    "armonik.api.grpc.v1.agent.CreateResultsResponse",
    "armonik.api.grpc.v1.agent.DataRequest",
    "armonik.api.grpc.v1.agent.DataResponse",
    "armonik.api.grpc.v1.agent.NotifyResultDataRequest",
    "armonik.api.grpc.v1.agent.NotifyResultDataRequest.ResultIdentifier",
    "armonik.api.grpc.v1.agent.NotifyResultDataResponse",
    "armonik.api.grpc.v1.agent.ResultMetaData",
    "armonik.api.grpc.v1.agent.SubmitTasksRequest",
    "armonik.api.grpc.v1.agent.SubmitTasksRequest.TaskCreation",
    "armonik.api.grpc.v1.agent.SubmitTasksResponse",
    "armonik.api.grpc.v1.agent.SubmitTasksResponse.TaskInfo",
    "armonik.api.grpc.v1.applications.ApplicationField",
    "armonik.api.grpc.v1.applications.ApplicationRawField",
    "armonik.api.grpc.v1.events.EventSubscriptionRequest",
    "armonik.api.grpc.v1.events.EventSubscriptionResponse",
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.NewResult",
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.NewTask",
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultOwnerUpdate",
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultStatusUpdate",
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.TaskStatusUpdate",
    "armonik.api.grpc.v1.partitions.FilterField",
    "armonik.api.grpc.v1.partitions.Filters",
    "armonik.api.grpc.v1.partitions.FiltersAnd",
    "armonik.api.grpc.v1.partitions.GetPartitionRequest",
    "armonik.api.grpc.v1.partitions.GetPartitionResponse",
    "armonik.api.grpc.v1.partitions.ListPartitionsRequest",
    "armonik.api.grpc.v1.partitions.ListPartitionsRequest.Sort",
    "armonik.api.grpc.v1.partitions.ListPartitionsResponse",
    "armonik.api.grpc.v1.partitions.PartitionField",
    "armonik.api.grpc.v1.partitions.PartitionRaw",
    "armonik.api.grpc.v1.partitions.PartitionRawField",
    "armonik.api.grpc.v1.results.CreateResultsMetaDataRequest",
    "armonik.api.grpc.v1.results.CreateResultsMetaDataRequest.ResultCreate",
    "armonik.api.grpc.v1.results.CreateResultsMetaDataResponse",
    "armonik.api.grpc.v1.results.CreateResultsRequest",
    "armonik.api.grpc.v1.results.CreateResultsRequest.ResultCreate",
    "armonik.api.grpc.v1.results.CreateResultsResponse",
    "armonik.api.grpc.v1.results.DeleteResultsDataRequest",
    "armonik.api.grpc.v1.results.DeleteResultsDataResponse",
    "armonik.api.grpc.v1.results.DownloadResultDataRequest",
    "armonik.api.grpc.v1.results.DownloadResultDataResponse",
    "armonik.api.grpc.v1.results.FilterField",
    "armonik.api.grpc.v1.results.Filters",
    "armonik.api.grpc.v1.results.FiltersAnd",
    "armonik.api.grpc.v1.results.GetOwnerTaskIdRequest",
    "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse",
    "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse.MapResultTask",
    "armonik.api.grpc.v1.results.GetResultRequest",
    "armonik.api.grpc.v1.results.GetResultResponse",
    "armonik.api.grpc.v1.results.ImportResultsDataRequest",
    "armonik.api.grpc.v1.results.ImportResultsDataRequest.ResultOpaqueId",
    "armonik.api.grpc.v1.results.ImportResultsDataResponse",
    "armonik.api.grpc.v1.results.ListResultsRequest",
    "armonik.api.grpc.v1.results.ListResultsRequest.Sort",
    "armonik.api.grpc.v1.results.ListResultsResponse",
    "armonik.api.grpc.v1.results.ResultField",
    "armonik.api.grpc.v1.results.ResultRaw",
    "armonik.api.grpc.v1.results.ResultRawField",
    "armonik.api.grpc.v1.results.ResultsServiceConfigurationResponse",
    "armonik.api.grpc.v1.results.UploadResultDataRequest",
    "armonik.api.grpc.v1.results.UploadResultDataRequest.ResultIdentifier",
    "armonik.api.grpc.v1.results.UploadResultDataResponse",
    "armonik.api.grpc.v1.results.WatchResultRequest",
    "armonik.api.grpc.v1.results.WatchResultResponse",
    "armonik.api.grpc.v1.sessions.CancelSessionRequest",
    "armonik.api.grpc.v1.sessions.CancelSessionResponse",
    "armonik.api.grpc.v1.sessions.CloseSessionRequest",
    "armonik.api.grpc.v1.sessions.CloseSessionResponse",
    "armonik.api.grpc.v1.sessions.CreateSessionReply",
    "armonik.api.grpc.v1.sessions.CreateSessionRequest",
    "armonik.api.grpc.v1.sessions.DeleteSessionRequest",
    "armonik.api.grpc.v1.sessions.DeleteSessionResponse",
    "armonik.api.grpc.v1.sessions.FilterField",
    "armonik.api.grpc.v1.sessions.Filters",
    "armonik.api.grpc.v1.sessions.FiltersAnd",
    "armonik.api.grpc.v1.sessions.GetSessionRequest",
    "armonik.api.grpc.v1.sessions.GetSessionResponse",
    "armonik.api.grpc.v1.sessions.ListSessionsRequest",
    "armonik.api.grpc.v1.sessions.ListSessionsRequest.Sort",
    "armonik.api.grpc.v1.sessions.ListSessionsResponse",
    "armonik.api.grpc.v1.sessions.PauseSessionRequest",
    "armonik.api.grpc.v1.sessions.PauseSessionResponse",
    "armonik.api.grpc.v1.sessions.PurgeSessionRequest",
    "armonik.api.grpc.v1.sessions.PurgeSessionResponse",
    "armonik.api.grpc.v1.sessions.ResumeSessionRequest",
    "armonik.api.grpc.v1.sessions.ResumeSessionResponse",
    "armonik.api.grpc.v1.sessions.SessionField",
    "armonik.api.grpc.v1.sessions.SessionRaw",
    "armonik.api.grpc.v1.sessions.SessionRawField",
    "armonik.api.grpc.v1.sessions.StopSubmissionRequest",
    "armonik.api.grpc.v1.sessions.StopSubmissionResponse",
    "armonik.api.grpc.v1.sessions.TaskOptionGenericField",
    "armonik.api.grpc.v1.submitter.AvailabilityReply",
    "armonik.api.grpc.v1.submitter.CreateLargeTaskRequest",
    "armonik.api.grpc.v1.submitter.CreateLargeTaskRequest.InitRequest",
    "armonik.api.grpc.v1.submitter.CreateSessionReply",
    "armonik.api.grpc.v1.submitter.CreateSessionRequest",
    "armonik.api.grpc.v1.submitter.CreateSmallTaskRequest",
    "armonik.api.grpc.v1.submitter.CreateTaskReply",
    "armonik.api.grpc.v1.submitter.CreateTaskReply.CreationStatus",
    "armonik.api.grpc.v1.submitter.CreateTaskReply.CreationStatusList",
    "armonik.api.grpc.v1.submitter.CreateTaskReply.TaskInfo",
    "armonik.api.grpc.v1.submitter.GetResultStatusReply",
    "armonik.api.grpc.v1.submitter.GetResultStatusReply.IdStatus",
    "armonik.api.grpc.v1.submitter.GetResultStatusRequest",
    "armonik.api.grpc.v1.submitter.GetTaskStatusReply",
    "armonik.api.grpc.v1.submitter.GetTaskStatusReply.IdStatus",
    "armonik.api.grpc.v1.submitter.GetTaskStatusRequest",
    "armonik.api.grpc.v1.submitter.ResultReply",
    "armonik.api.grpc.v1.submitter.SessionFilter",
    "armonik.api.grpc.v1.submitter.SessionFilter.StatusesRequest",
    "armonik.api.grpc.v1.submitter.SessionIdList",
    "armonik.api.grpc.v1.submitter.SessionList",
    "armonik.api.grpc.v1.submitter.TaskFilter",
    "armonik.api.grpc.v1.submitter.TaskFilter.IdsRequest",
    "armonik.api.grpc.v1.submitter.TaskFilter.StatusesRequest",
    "armonik.api.grpc.v1.submitter.WaitRequest",
    "armonik.api.grpc.v1.submitter.WatchResultRequest",
    "armonik.api.grpc.v1.submitter.WatchResultStream",
    "armonik.api.grpc.v1.tasks.CancelTasksRequest",
    "armonik.api.grpc.v1.tasks.CancelTasksResponse",
    "armonik.api.grpc.v1.tasks.CountTasksByStatusRequest",
    "armonik.api.grpc.v1.tasks.CountTasksByStatusResponse",
    "armonik.api.grpc.v1.tasks.FilterField",
    "armonik.api.grpc.v1.tasks.Filters",
    "armonik.api.grpc.v1.tasks.FiltersAnd",
    "armonik.api.grpc.v1.tasks.GetResultIdsRequest",
    "armonik.api.grpc.v1.tasks.GetResultIdsResponse",
    "armonik.api.grpc.v1.tasks.GetResultIdsResponse.MapTaskResult",
    "armonik.api.grpc.v1.tasks.GetTaskRequest",
    "armonik.api.grpc.v1.tasks.GetTaskResponse",
    "armonik.api.grpc.v1.tasks.ListTasksDetailedResponse",
    "armonik.api.grpc.v1.tasks.ListTasksRequest",
    "armonik.api.grpc.v1.tasks.ListTasksRequest.Sort",
    "armonik.api.grpc.v1.tasks.ListTasksResponse",
    "armonik.api.grpc.v1.tasks.SubmitTasksRequest",
    "armonik.api.grpc.v1.tasks.SubmitTasksRequest.TaskCreation",
    "armonik.api.grpc.v1.tasks.SubmitTasksResponse",
    "armonik.api.grpc.v1.tasks.SubmitTasksResponse.TaskInfo",
    "armonik.api.grpc.v1.tasks.TaskDetailed",
    "armonik.api.grpc.v1.tasks.TaskDetailed.Output",
    "armonik.api.grpc.v1.tasks.TaskField",
    "armonik.api.grpc.v1.tasks.TaskOptionGenericField",
    "armonik.api.grpc.v1.tasks.TaskSummary",
    "armonik.api.grpc.v1.tasks.TaskSummaryField",
    "armonik.api.grpc.v1.worker.HealthCheckReply",
    "armonik.api.grpc.v1.worker.ProcessReply",
    "armonik.api.grpc.v1.worker.ProcessRequest",
];

#[test]
fn descriptor_coverage_ratchet() {
    let pool = pool();
    let registered: Vec<&str> = registry::entries()
        .into_iter()
        .map(|entry| entry.proto)
        .collect();

    let mut missing = Vec::new();
    for message in pool.all_messages() {
        let name = message.full_name();
        if !name.starts_with("armonik.") || message.is_map_entry() {
            continue;
        }
        if registered.contains(&name)
            || PERMANENT_UNMAPPED.contains(&name)
            || TEMP_UNMAPPED.contains(&name)
        {
            continue;
        }
        missing.push(name.to_owned());
    }
    missing.sort();
    assert!(
        missing.is_empty(),
        "messages neither mapped nor tracked; add them to the registry or TEMP_UNMAPPED:\n    \"{}\"",
        missing.join("\",\n    \"")
    );

    // Ratchet: entries must leave TEMP_UNMAPPED when they become mapped,
    // and every tracked name must actually exist.
    for name in TEMP_UNMAPPED {
        assert!(
            pool.get_message_by_name(name).is_some(),
            "TEMP_UNMAPPED entry `{name}` does not exist in the descriptor"
        );
        assert!(
            !registered.contains(name),
            "`{name}` is registered; remove it from TEMP_UNMAPPED"
        );
    }
    for name in PERMANENT_UNMAPPED {
        assert!(
            pool.get_message_by_name(name).is_some(),
            "PERMANENT_UNMAPPED entry `{name}` does not exist in the descriptor"
        );
    }
}
