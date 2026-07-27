//! Discovery of the proto-to-type mapping, and the projection of messages
//! onto the armonik types' documented equivalence classes.
//!
//! The mapping is self-registering: every derived (and hand-written)
//! message type pushes an [`Entry`] into `armonik_types::differential::REGISTRY`
//! under the private `_differential` feature, so new messages are covered
//! without touching the harness. Each entry carries the type's own
//! `Normalize` projection, generated from the same constructs that shape
//! its codec (adapters, markers, wrapper chains) or hand-written next to
//! the hand-written impls. Only the generic instantiations below are
//! hand-maintained — the derive has no proto name for them.

use std::collections::HashMap;
use std::sync::OnceLock;

use armonik_types::reexports::prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage, Value};

pub use armonik_types::differential::Entry;

macro_rules! generic_instantiations {
    ($($proto:literal => $ty:ty),* $(,)?) => {
        static GENERIC_INSTANTIATIONS: &[Entry] = &[$(Entry {
            proto: $proto,
            roundtrip: |bytes| Ok(<$ty as Message>::decode(bytes)?.encode_to_vec()),
            default_encoding: || <$ty as Default>::default().encode_to_vec(),
            normalize: <$ty as armonik_types::differential::Normalize>::normalize,
        }),*];
    };
}

generic_instantiations! {
    "armonik.api.grpc.v1.applications.ListApplicationsRequest.Sort" => armonik_types::applications::Sort,
    "armonik.api.grpc.v1.partitions.ListPartitionsRequest.Sort" => armonik_types::partitions::Sort,
    "armonik.api.grpc.v1.sessions.ListSessionsRequest.Sort" => armonik_types::sessions::Sort,
    "armonik.api.grpc.v1.tasks.ListTasksRequest.Sort" => armonik_types::tasks::Sort,
    "armonik.api.grpc.v1.results.ListResultsRequest.Sort" => armonik_types::results::Sort,
    "armonik.api.grpc.v1.sessions.FilterStatus"
        => armonik_types::FilterStatus<armonik_types::SessionStatus>,
    "armonik.api.grpc.v1.tasks.FilterStatus"
        => armonik_types::FilterStatus<armonik_types::TaskStatus>,
    "armonik.api.grpc.v1.results.FilterStatus"
        => armonik_types::FilterStatus<armonik_types::ResultStatus>,
}

pub fn entries() -> impl Iterator<Item = &'static Entry> {
    armonik_types::differential::REGISTRY
        .iter()
        .chain(GENERIC_INSTANTIATIONS)
}

/// Project a message (recursively) onto the equivalence classes of its
/// armonik type, so that the semantic comparison reflects the documented
/// semantics. Applied to both sides of every round-trip.
pub fn normalize(message: &mut DynamicMessage) {
    let descriptor = message.descriptor();
    for field in descriptor.fields() {
        if !message.has_field(&field) {
            continue;
        }
        let mut value = message.get_field(&field).into_owned();
        if normalize_value(&mut value) {
            message.set_field(&field, value);
        }
    }
    apply_rules(message);
}

fn normalize_value(value: &mut Value) -> bool {
    match value {
        Value::Message(inner) => {
            normalize(inner);
            true
        }
        Value::List(items) => {
            let mut changed = false;
            for item in items {
                changed |= normalize_value(item);
            }
            changed
        }
        Value::Map(map) => {
            let mut changed = false;
            for item in map.values_mut() {
                changed |= normalize_value(item);
            }
            changed
        }
        _ => false,
    }
}

/// Canonical "absent" form per proto name: what each type emits for
/// `Default::default()`, decoded back dynamically. This is the harness's
/// only own contribution to the quotient, derived from the implementation
/// itself instead of restated: whatever a type emits for "nothing" is what
/// an absent field or member is equivalent to.
fn canonicals() -> &'static HashMap<&'static str, (DynamicMessage, &'static Entry)> {
    static CANONICALS: OnceLock<HashMap<&'static str, (DynamicMessage, &'static Entry)>> =
        OnceLock::new();
    CANONICALS.get_or_init(|| {
        let pool = crate::pool();
        let mut map = HashMap::new();
        for entry in entries() {
            let Some(desc) = pool.get_message_by_name(entry.proto) else {
                continue;
            };
            let canonical = DynamicMessage::decode(desc, (entry.default_encoding)().as_slice())
                .expect("the default encoding decodes");
            map.entry(entry.proto).or_insert((canonical, entry));
        }
        map
    })
}

fn apply_rules(message: &mut DynamicMessage) {
    let name = message.descriptor().full_name().to_owned();
    let Some((canonical, entry)) = canonicals().get(name.as_str()) else {
        return;
    };
    // The value projections declared by the type itself.
    (entry.normalize)(message);
    // The canonical-absence fold: whatever the type emits for "nothing"
    // (default oneof members, always-emitted wrappers and sorts)
    // materializes where it is absent — except into a oneof that
    // already carries another member.
    for member in canonical.descriptor().fields() {
        if !canonical.has_field(&member) || message.has_field(&member) {
            continue;
        }
        if let Some(oneof) = member.containing_oneof() {
            if oneof.fields().any(|other| message.has_field(&other)) {
                continue;
            }
        }
        message.set_field(&member, canonical.get_field(&member).into_owned());
    }
}
