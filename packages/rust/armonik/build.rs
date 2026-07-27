use std::collections::BTreeSet;
use std::error::Error;

use prost::Message;

/// Extern types that cannot be harvested from the `#[armonik(message = ...)]`
/// annotations, so they are spelled out here:
///
/// - the five synthetic per-site empty messages injected by `prune_for_stubs`
///   (see `EMPTY_SIGNATURES`): their real annotation is `Empty`, one name
///   standing for five distinct API types, so the harvested map carries them
///   ambiguously keyed under `Empty` and the build filters those out;
/// - the generic sort / filter-status instantiations, which are type aliases
///   of `SortMany<T>` / `FilterStatus<T>` and carry no annotation of their own
///   (they are hand-registered in the differential harness the same way).
///
/// Everything else — ~150 messages — comes from `armonik_types::wire`.
const EXTRA_EXTERN_TYPES: &[(&str, &str)] = &[
    (
        ".armonik.api.grpc.v1.worker.HealthCheckRequest",
        "::armonik_types::worker::health_check::Request",
    ),
    (
        ".armonik.api.grpc.v1.results.GetServiceConfigurationRequest",
        "::armonik_types::results::get_service_configuration::Request",
    ),
    (
        ".armonik.api.grpc.v1.submitter.GetServiceConfigurationRequest",
        "::armonik_types::submitter::get_service_configuration::Request",
    ),
    (
        ".armonik.api.grpc.v1.submitter.CancelSessionResponse",
        "::armonik_types::submitter::cancel_session::Response",
    ),
    (
        ".armonik.api.grpc.v1.submitter.CancelTasksResponse",
        "::armonik_types::submitter::cancel_tasks::Response",
    ),
    (
        ".armonik.api.grpc.v1.applications.ListApplicationsRequest.Sort",
        "::armonik_types::applications::Sort",
    ),
    (
        ".armonik.api.grpc.v1.partitions.ListPartitionsRequest.Sort",
        "::armonik_types::partitions::Sort",
    ),
    (
        ".armonik.api.grpc.v1.sessions.ListSessionsRequest.Sort",
        "::armonik_types::sessions::Sort",
    ),
    (
        ".armonik.api.grpc.v1.tasks.ListTasksRequest.Sort",
        "::armonik_types::tasks::Sort",
    ),
    (
        ".armonik.api.grpc.v1.results.ListResultsRequest.Sort",
        "::armonik_types::results::Sort",
    ),
    (
        ".armonik.api.grpc.v1.sessions.FilterStatus",
        "::armonik_types::sessions::filter::Status",
    ),
    (
        ".armonik.api.grpc.v1.tasks.FilterStatus",
        "::armonik_types::tasks::filter::Status",
    ),
    (
        ".armonik.api.grpc.v1.results.FilterStatus",
        "::armonik_types::results::filter::Status",
    ),
];

/// RPC methods excluded from the generated stubs: the crate does not expose
/// them, and tonic answers UNIMPLEMENTED for unrouted paths, so pruning them
/// is behaviorally identical to the unimplemented hand-written stubs it
/// replaces.
const PRUNED_METHODS: &[(&str, &str)] =
    &[("Results", "WatchResults"), ("Submitter", "WatchResults")];

/// Wire-compatible signature rewrites: the five RPC signatures using
/// `Empty` stand for five distinct API types. Message type names never
/// appear on the wire, so the stub descriptor references a distinct
/// synthetic empty message per site (injected below and extern'd to the
/// API type in `EXTRA_EXTERN_TYPES`).
const EMPTY_SIGNATURES: &[(&str, &str, Direction, &str)] = &[
    (
        "Worker",
        "HealthCheck",
        Direction::Input,
        "armonik.api.grpc.v1.worker.HealthCheckRequest",
    ),
    (
        "Results",
        "GetServiceConfiguration",
        Direction::Input,
        "armonik.api.grpc.v1.results.GetServiceConfigurationRequest",
    ),
    (
        "Submitter",
        "GetServiceConfiguration",
        Direction::Input,
        "armonik.api.grpc.v1.submitter.GetServiceConfigurationRequest",
    ),
    (
        "Submitter",
        "CancelSession",
        Direction::Output,
        "armonik.api.grpc.v1.submitter.CancelSessionResponse",
    ),
    (
        "Submitter",
        "CancelTasks",
        Direction::Output,
        "armonik.api.grpc.v1.submitter.CancelTasksResponse",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    Input,
    Output,
}

/// Messages excluded from the stub generation: nothing generated references
/// them — they are field wrappers flattened into armonik enums, messages of
/// the pruned RPCs, or unused legacy. Together with the extern types this
/// leaves the generated module with the client/server stubs only.
/// `armonik_types`' `descriptor.bin` keeps the full set for the derives and
/// the harness.
const PRUNED_MESSAGES: &[&str] = &[
    "armonik.api.grpc.v1.Empty",
    "armonik.api.grpc.v1.applications.ApplicationField",
    "armonik.api.grpc.v1.applications.ApplicationRawField",
    "armonik.api.grpc.v1.partitions.PartitionField",
    "armonik.api.grpc.v1.partitions.PartitionRawField",
    "armonik.api.grpc.v1.sessions.SessionField",
    "armonik.api.grpc.v1.sessions.SessionRawField",
    "armonik.api.grpc.v1.sessions.TaskOptionField",
    "armonik.api.grpc.v1.sessions.TaskOptionGenericField",
    "armonik.api.grpc.v1.tasks.TaskField",
    "armonik.api.grpc.v1.tasks.TaskSummaryField",
    "armonik.api.grpc.v1.tasks.TaskOptionField",
    "armonik.api.grpc.v1.tasks.TaskOptionGenericField",
    "armonik.api.grpc.v1.results.ResultField",
    "armonik.api.grpc.v1.results.ResultRawField",
    "armonik.api.grpc.v1.results.WatchResultRequest",
    "armonik.api.grpc.v1.results.WatchResultResponse",
    "armonik.api.grpc.v1.submitter.SessionList",
    "armonik.api.grpc.v1.submitter.WatchResultRequest",
    "armonik.api.grpc.v1.submitter.WatchResultStream",
];

/// Stub-generation copy of the descriptor set: without the pruned methods
/// and messages, and without the file-level enums (every remaining message
/// is extern'd, so no generated code can reference them). Unknown names in
/// the prune lists are an error, so they cannot go stale silently.
fn prune_for_stubs(
    mut fds: prost_types::FileDescriptorSet,
) -> Result<prost_types::FileDescriptorSet, Box<dyn Error>> {
    let mut methods: Vec<(&str, &str)> = PRUNED_METHODS.to_vec();
    let mut messages: Vec<&str> = PRUNED_MESSAGES.to_vec();
    let mut rewrites: Vec<&(&str, &str, Direction, &str)> = EMPTY_SIGNATURES.iter().collect();
    for file in &mut fds.file {
        if !file.package().starts_with("armonik.") {
            continue;
        }
        let package = file.package().to_owned();
        for service in &mut file.service {
            let service_name = service.name().to_owned();
            service.method.retain(|method| {
                let position = methods.iter().position(|(service, method_name)| {
                    *service == service_name && *method_name == method.name()
                });
                match position {
                    Some(position) => {
                        methods.swap_remove(position);
                        false
                    }
                    None => true,
                }
            });
            for method in &mut service.method {
                let position = rewrites.iter().position(|(service, method_name, _, _)| {
                    *service == service_name && *method_name == method.name()
                });
                let Some(position) = position else { continue };
                let (_, _, direction, new_name) = rewrites.swap_remove(position);
                let method_name = method.name().to_owned();
                let slot = match direction {
                    Direction::Input => &mut method.input_type,
                    Direction::Output => &mut method.output_type,
                };
                if slot.as_deref() != Some(".armonik.api.grpc.v1.Empty") {
                    return Err(format!(
                        "signature of {service_name}.{method_name} no longer uses Empty \
                         ({slot:?})",
                    )
                    .into());
                }
                *slot = Some(format!(".{new_name}"));
            }
        }
        // Inject the synthetic empty messages whose package is this file's.
        for (_, _, _, new_name) in EMPTY_SIGNATURES {
            let (message_package, name) = new_name.rsplit_once('.').expect("qualified name");
            if message_package == package {
                file.message_type.push(prost_types::DescriptorProto {
                    name: Some(name.to_owned()),
                    ..Default::default()
                });
            }
        }
        file.message_type.retain(|message| {
            let full_name = format!("{package}.{}", message.name());
            match messages.iter().position(|name| *name == full_name) {
                Some(position) => {
                    messages.swap_remove(position);
                    false
                }
                None => true,
            }
        });
        file.enum_type.clear();
    }
    if !methods.is_empty() || !messages.is_empty() || !rewrites.is_empty() {
        return Err(format!(
            "stale prune/rewrite entries (not found in the descriptor set): \
             {methods:?} {messages:?} {rewrites:?}",
        )
        .into());
    }
    Ok(fds)
}

/// Every top-level message left in the pruned descriptor must be extern'd:
/// externing a message suppresses its generation and that of its nested
/// types, so if all top-level messages are extern'd the generated module
/// carries the client/server stubs and nothing else. A message that is
/// neither extern'd nor pruned would materialize as a generated struct — the
/// ratchet that keeps the harvested map honest as the schema evolves.
fn guard_all_messages_externed(
    fds: &prost_types::FileDescriptorSet,
    extern_types: &BTreeSet<&str>,
) -> Result<(), Box<dyn Error>> {
    let mut orphans = Vec::new();
    for file in &fds.file {
        if !file.package().starts_with("armonik.") {
            continue;
        }
        for message in &file.message_type {
            let full_name = format!(".{}.{}", file.package(), message.name());
            if !extern_types.contains(full_name.as_str()) {
                orphans.push(full_name);
            }
        }
    }
    if !orphans.is_empty() {
        orphans.sort();
        return Err(format!(
            "these messages survive stub pruning but are not extern'd, so they would be \
             generated as structs; annotate the type (it will be harvested automatically), \
             add it to PRUNED_MESSAGES, or add it to EXTRA_EXTERN_TYPES:\n    {}",
            orphans.join("\n    "),
        )
        .into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // The descriptor and the annotation-harvested extern map are pulled from
    // `armonik-types`, compiled first as a build-dependency; no proto files
    // are compiled here.
    let fds = prost_types::FileDescriptorSet::decode(armonik_types::wire::DESCRIPTOR)?;

    // Extern map: the harvested `(proto name, Rust path)` pairs, normalized to
    // the fully-qualified `.proto.Name` / `::rust::Path` forms prost expects,
    // with the ambiguous `Empty`-keyed synthetic entries dropped, plus the
    // handful that cannot come from annotations.
    let harvested: Vec<(String, String)> = armonik_types::wire::extern_mapping()
        .into_iter()
        .filter(|(proto, _)| *proto != "armonik.api.grpc.v1.Empty")
        .map(|(proto, path)| (format!(".{proto}"), format!("::{path}")))
        .collect();
    let extern_types: Vec<(&str, &str)> = harvested
        .iter()
        .map(|(proto, path)| (proto.as_str(), path.as_str()))
        .chain(EXTRA_EXTERN_TYPES.iter().copied())
        .collect();

    let pruned = prune_for_stubs(fds)?;

    let extern_names: BTreeSet<&str> = extern_types.iter().map(|(proto, _)| *proto).collect();
    guard_all_messages_externed(&pruned, &extern_names)?;

    // Generate the tonic stubs from the pruned descriptor set: with every
    // extern'd message resolved to its armonik type and the unreferenced ones
    // pruned, the generated module contains nothing but the stubs.
    let mut builder = tonic_prost_build::configure()
        .use_arc_self(true)
        .build_client(cfg!(feature = "_gen-client"))
        .build_server(cfg!(feature = "_gen-server"));
    for (proto_path, rust_path) in &extern_types {
        builder = builder.extern_path(*proto_path, *rust_path);
    }
    builder.compile_fds(pruned)?;

    Ok(())
}
