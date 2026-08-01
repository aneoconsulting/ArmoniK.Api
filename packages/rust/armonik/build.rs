use std::collections::{BTreeSet, HashSet};
use std::error::Error;

use armonik_types::wire::{Direction, Replacement};
use prost::Message;

/// RPC methods excluded from the generated stubs: the crate does not expose
/// them, and tonic answers UNIMPLEMENTED for unrouted paths, so pruning them
/// is behaviorally identical to the unimplemented hand-written stubs it
/// replaces.
const PRUNED_METHODS: &[(&str, &str)] =
    &[("Results", "WatchResults"), ("Submitter", "WatchResults")];

// The type-less messages of RPCs the crate does not expose are pruned from one
// shared list, `armonik_types::wire::UNEXPOSED_RPC_MESSAGES` (also the
// differential coverage ratchet's allowlist, so the two cannot drift). The
// other type-less messages are pruned without a list entry in `prune_for_stubs`:
// shared messages a `#[armonik(replace(...))]` takes over (`Empty`, `TaskFilter`,
// …) once every use is rewritten to a synthetic, and messages a flattening
// construct absorbs (the `*Field` selectors, pair entries, …) through
// `armonik_types::wire::absorbed()`.

/// Every message name still used as an RPC input/output slot, fully qualified
/// with a leading `.`. Message *fields* are deliberately ignored: every
/// surviving message is extern'd (the guard enforces it), so tonic never
/// generates one nor reads its fields — only an RPC slot forces a type to be
/// present and extern'd. A replaced message no longer named by any slot can
/// therefore be dropped even if some extern'd message still has a field of
/// that type (e.g. `Output.ok: Empty`).
fn referenced_by_rpc(fds: &prost_types::FileDescriptorSet) -> HashSet<String> {
    let mut refs = HashSet::new();
    for file in &fds.file {
        for service in &file.service {
            for method in &service.method {
                if let Some(name) = &method.input_type {
                    refs.insert(name.clone());
                }
                if let Some(name) = &method.output_type {
                    refs.insert(name.clone());
                }
            }
        }
    }
    refs
}

/// Stub-generation copy of the descriptor set: never-exposed methods and
/// messages removed, per-RPC message substitutions applied, and file-level
/// enums dropped (every remaining message is extern'd, so no generated code can
/// reference them). Runs three sequenced passes; pass order is call order.
/// Unknown names in the prune lists — and replacements whose RPC or expected
/// message no longer match the descriptor — are errors, so they cannot go stale
/// silently.
fn prune_for_stubs(
    mut fds: prost_types::FileDescriptorSet,
    replacements: &[&Replacement],
) -> Result<prost_types::FileDescriptorSet, Box<dyn Error>> {
    prune_methods(&mut fds)?;
    apply_replacements(&mut fds, replacements)?;
    prune_messages(&mut fds, replacements)?;
    Ok(fds)
}

/// Pass 1: drop the never-exposed RPC methods (`PRUNED_METHODS`).
fn prune_methods(fds: &mut prost_types::FileDescriptorSet) -> Result<(), Box<dyn Error>> {
    let mut methods: Vec<(&str, &str)> = PRUNED_METHODS.to_vec();
    for file in &mut fds.file {
        if !file.package().starts_with("armonik.") {
            continue;
        }
        for service in &mut file.service {
            let service_name = service.name().to_owned();
            service.method.retain(|method| {
                match methods
                    .iter()
                    .position(|(service, name)| *service == service_name && *name == method.name())
                {
                    Some(position) => {
                        methods.swap_remove(position);
                        false
                    }
                    None => true,
                }
            });
        }
    }
    if !methods.is_empty() {
        return Err(format!("stale PRUNED_METHODS (not in the descriptor set): {methods:?}").into());
    }
    Ok(())
}

/// Pass 2: rewrite each `#[armonik(replace(...))]` RPC slot to its synthetic
/// target message and inject that (empty) message, drift-checking the slot
/// against the live descriptor first. A replacement that never matched a slot,
/// or whose target has no package file, is stale.
fn apply_replacements(
    fds: &mut prost_types::FileDescriptorSet,
    replacements: &[&Replacement],
) -> Result<(), Box<dyn Error>> {
    let mut applied = vec![false; replacements.len()];
    let mut injected = vec![false; replacements.len()];

    for file in &mut fds.file {
        if !file.package().starts_with("armonik.") {
            continue;
        }
        let package = file.package().to_owned();
        for service in &mut file.service {
            let service_name = service.name().to_owned();
            for method in &mut service.method {
                let method_name = method.name().to_owned();
                for (index, replacement) in replacements.iter().enumerate() {
                    if replacement.service != service_name || replacement.method != method_name {
                        continue;
                    }
                    let expected = format!(".{}", replacement.message);
                    let slot = match replacement.direction {
                        Direction::Input => &mut method.input_type,
                        Direction::Output => &mut method.output_type,
                    };
                    // The consistency check: the RPC slot must still hold the
                    // message the type declares it replaces.
                    if slot.as_deref() != Some(expected.as_str()) {
                        return Err(format!(
                            "replace: {service_name}.{method_name} {direction:?} is {slot:?}, \
                             but `{rust}` declares it replaces `{expected}`",
                            direction = replacement.direction,
                            rust = replacement.rust_path,
                        )
                        .into());
                    }
                    *slot = Some(format!(".{}", replacement.target));
                    applied[index] = true;
                }
            }
        }
        // Inject the synthetic target messages whose package is this file's.
        for (index, replacement) in replacements.iter().enumerate() {
            let (target_package, name) =
                replacement.target.rsplit_once('.').expect("qualified name");
            if target_package != package {
                continue;
            }
            if file.message_type.iter().any(|message| message.name() == name) {
                return Err(format!(
                    "replace target `{}` collides with an existing message",
                    replacement.target
                )
                .into());
            }
            file.message_type.push(prost_types::DescriptorProto {
                name: Some(name.to_owned()),
                ..Default::default()
            });
            injected[index] = true;
        }
    }

    for (index, replacement) in replacements.iter().enumerate() {
        if !applied[index] {
            return Err(format!(
                "stale replace on `{}`: no {:?} slot of {}.{} to rewrite",
                replacement.rust_path,
                replacement.direction,
                replacement.service,
                replacement.method,
            )
            .into());
        }
        if !injected[index] {
            return Err(format!(
                "replace target `{}` has no matching package file to inject into",
                replacement.target,
            )
            .into());
        }
    }
    Ok(())
}

/// Pass 3: drop the type-less messages and clear file-level enums. A message is
/// dropped when it is a never-exposed RPC message (`UNEXPOSED_RPC_MESSAGES`), a
/// flattening construct absorbs it (`wire::absorbed()`), or it is a replaced
/// shared message no surviving RPC slot names anymore (`Empty`, the legacy
/// filters); a replaced message still used elsewhere (a field, or an unreplaced
/// RPC sharing the type) stays and keeps its canonical extern mapping.
/// `referenced_by_rpc` is computed here, after pass 2 rewrote the slots.
fn prune_messages(
    fds: &mut prost_types::FileDescriptorSet,
    replacements: &[&Replacement],
) -> Result<(), Box<dyn Error>> {
    let mut messages: Vec<&str> = armonik_types::wire::UNEXPOSED_RPC_MESSAGES.to_vec();
    let referenced = referenced_by_rpc(fds);
    let replaced: HashSet<String> = replacements
        .iter()
        .map(|replacement| format!(".{}", replacement.message))
        .collect();
    let absorbed: HashSet<String> = armonik_types::wire::absorbed()
        .into_iter()
        .map(|name| format!(".{name}"))
        .collect();

    for file in &mut fds.file {
        if !file.package().starts_with("armonik.") {
            continue;
        }
        let package = file.package().to_owned();
        file.message_type.retain(|message| {
            let full_name = format!("{package}.{}", message.name());
            if let Some(position) = messages.iter().position(|name| *name == full_name) {
                messages.swap_remove(position);
                return false;
            }
            let dotted = format!(".{full_name}");
            if absorbed.contains(&dotted) {
                return false;
            }
            // Keep unless it is a replaced message no RPC slot names anymore.
            !replaced.contains(&dotted) || referenced.contains(&dotted)
        });
        file.enum_type.clear();
    }

    if !messages.is_empty() {
        return Err(
            format!("stale UNEXPOSED_RPC_MESSAGES (not in the descriptor set): {messages:?}").into(),
        );
    }
    Ok(())
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
             add it to PRUNED_MESSAGES, or give it a #[armonik(replace(...))]:\n    {}",
            orphans.join("\n    "),
        )
        .into());
    }
    Ok(())
}

/// Exactly one Rust type may stand for each proto message. Two distinct types
/// claiming one proto name (two shared-message siblings that both forgot to
/// carry `#[armonik(replace(...))]`) would each `extern_path` the same name and
/// tonic would silently keep the last — a misbinding no other check catches,
/// because `extern_mapping()` only collapses *identical* pairs. See the 1-of-N
/// convention on `armonik_types::wire::Role::Replace`.
fn guard_unique_extern(extern_types: &[(&str, &str)]) -> Result<(), Box<dyn Error>> {
    let mut by_proto: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for &(proto, rust) in extern_types {
        if let Some(previous) = by_proto.insert(proto, rust) {
            if previous != rust {
                return Err(format!(
                    "proto message `{proto}` is extern-mapped to two Rust types \
                     (`{previous}` and `{rust}`); exactly one type may stand for a shared \
                     wire message — give the others a #[armonik(replace(...))]"
                )
                .into());
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // The descriptor, the annotation-harvested extern map and the per-RPC
    // replacements are pulled from `armonik-types`, compiled first as a
    // build-dependency; no proto files are compiled here.
    let fds = prost_types::FileDescriptorSet::decode(armonik_types::wire::DESCRIPTOR)?;
    let replacements = armonik_types::wire::replacements();

    // Extern map: the harvested `(proto name, Rust path)` pairs, normalized to
    // the fully-qualified `.proto.Name` / `::rust::Path` forms prost expects,
    // plus one entry per replacement mapping its synthetic target message to
    // the standing-in type.
    let harvested: Vec<(String, String)> = armonik_types::wire::extern_mapping()
        .into_iter()
        .map(|(proto, path)| (format!(".{proto}"), format!("::{path}")))
        .collect();
    let replacement_externs: Vec<(String, String)> = replacements
        .iter()
        .map(|replacement| {
            (
                format!(".{}", replacement.target),
                format!("::{}", replacement.rust_path),
            )
        })
        .collect();
    let extern_types: Vec<(&str, &str)> = harvested
        .iter()
        .map(|(proto, path)| (proto.as_str(), path.as_str()))
        .chain(
            replacement_externs
                .iter()
                .map(|(proto, path)| (proto.as_str(), path.as_str())),
        )
        .collect();
    guard_unique_extern(&extern_types)?;

    let pruned = prune_for_stubs(fds, &replacements)?;

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
