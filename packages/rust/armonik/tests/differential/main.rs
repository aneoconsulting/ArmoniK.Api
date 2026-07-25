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

            registry::normalize(&mut original, registry::Side::Original);
            registry::normalize(&mut back, registry::Side::Back);

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
    // Enum wrapper chain flattened into `partitions::Field`.
    "armonik.api.grpc.v1.partitions.PartitionField",
    "armonik.api.grpc.v1.partitions.PartitionRawField",
    // Enum wrapper flattened into `sessions::RawField`.
    "armonik.api.grpc.v1.sessions.SessionRawField",
    // String wrapper flattened into `sessions::Field::TaskOptionGeneric`.
    "armonik.api.grpc.v1.sessions.TaskOptionGenericField",
    // Enum wrapper flattened into `tasks::SummaryField`.
    "armonik.api.grpc.v1.tasks.TaskSummaryField",
    // String wrapper flattened into `tasks::Field::OptionGeneric`.
    "armonik.api.grpc.v1.tasks.TaskOptionGenericField",
    // Pair entries flattened into the `task_results` map.
    "armonik.api.grpc.v1.tasks.GetResultIdsResponse.MapTaskResult",
    // Enum wrapper chain flattened into `results::Field`.
    "armonik.api.grpc.v1.results.ResultField",
    "armonik.api.grpc.v1.results.ResultRawField",
    // Pair entries flattened into the `result_task` and `results` maps.
    "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse.MapResultTask",
    "armonik.api.grpc.v1.results.ImportResultsDataRequest.ResultOpaqueId",
    // Inlined into the `upload::Request::Identifier` struct variant.
    "armonik.api.grpc.v1.results.UploadResultDataRequest.ResultIdentifier",
    // The WatchResults RPC is not exposed by the crate.
    "armonik.api.grpc.v1.results.WatchResultRequest",
    "armonik.api.grpc.v1.results.WatchResultResponse",
    // Pair entries flattened into the shared session ID and result IDs.
    "armonik.api.grpc.v1.agent.NotifyResultDataRequest.ResultIdentifier",
    // Inlined into the `submitter::create_tasks::Status::TaskInfo` variant.
    "armonik.api.grpc.v1.submitter.CreateTaskReply.TaskInfo",
    // Flattened into `submitter::create_tasks::Response::Status`.
    "armonik.api.grpc.v1.submitter.CreateTaskReply.CreationStatusList",
    // Flattened into the filter variants through `VecWrapper`.
    "armonik.api.grpc.v1.submitter.TaskFilter.IdsRequest",
    "armonik.api.grpc.v1.submitter.TaskFilter.StatusesRequest",
    "armonik.api.grpc.v1.submitter.SessionFilter.StatusesRequest",
    // Pair entries flattened into the `statuses` maps.
    "armonik.api.grpc.v1.submitter.GetTaskStatusReply.IdStatus",
    "armonik.api.grpc.v1.submitter.GetResultStatusReply.IdStatus",
    // Not exposed by the crate.
    "armonik.api.grpc.v1.submitter.SessionList",
    "armonik.api.grpc.v1.submitter.WatchResultRequest",
    "armonik.api.grpc.v1.submitter.WatchResultStream",
];

/// Messages not yet migrated to a direct wire implementation. This list
/// only shrinks: annotating a type moves it to the registry, and the test
/// fails on stale entries. It must be empty by the end of the migration.
const TEMP_UNMAPPED: &[&str] = &[];

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
