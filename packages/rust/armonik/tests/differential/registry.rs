//! Discovery of the proto-to-type mapping, and the projection of messages
//! onto the armonik types' documented equivalence classes.
//!
//! The mapping is self-registering: every derived (and hand-written)
//! message type pushes an [`Entry`] into `armonik::differential::REGISTRY`
//! under the private `_differential` feature, so new messages are covered
//! without touching the harness. Only the generic instantiations below are
//! hand-maintained — the derive has no proto name for them.

use std::collections::HashMap;
use std::sync::OnceLock;

use armonik::reexports::prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage, Value};

pub use armonik::differential::Entry;

macro_rules! generic_instantiations {
    ($($proto:literal => $ty:ty),* $(,)?) => {
        static GENERIC_INSTANTIATIONS: &[Entry] = &[$(Entry {
            proto: $proto,
            roundtrip: |bytes| Ok(<$ty as Message>::decode(bytes)?.encode_to_vec()),
            default_encoding: || <$ty as Default>::default().encode_to_vec(),
            bool_markers: &[],
            wrapper_chain: false,
        }),*];
    };
}

generic_instantiations! {
    "armonik.api.grpc.v1.applications.ListApplicationsRequest.Sort" => armonik::applications::Sort,
    "armonik.api.grpc.v1.partitions.ListPartitionsRequest.Sort" => armonik::partitions::Sort,
    "armonik.api.grpc.v1.sessions.ListSessionsRequest.Sort" => armonik::sessions::Sort,
    "armonik.api.grpc.v1.tasks.ListTasksRequest.Sort" => armonik::tasks::Sort,
    "armonik.api.grpc.v1.results.ListResultsRequest.Sort" => armonik::results::Sort,
    "armonik.api.grpc.v1.sessions.FilterStatus"
        => armonik::FilterStatus<armonik::SessionStatus>,
    "armonik.api.grpc.v1.tasks.FilterStatus"
        => armonik::FilterStatus<armonik::TaskStatus>,
    "armonik.api.grpc.v1.results.FilterStatus"
        => armonik::FilterStatus<armonik::ResultStatus>,
}

pub fn entries() -> impl Iterator<Item = &'static Entry> {
    armonik::differential::REGISTRY
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
/// quotient, derived from the implementation itself instead of restated:
/// whatever a type emits for "nothing" is what an absent field or member
/// is equivalent to.
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
    if let Some((canonical, entry)) = canonicals().get(name.as_str()) {
        // Value projections declared by the type itself.
        for marker in entry.bool_markers {
            let member = field(message, marker);
            if message.has_field(&member) {
                message.set_field(&member, Value::Bool(true));
            }
        }
        if entry.wrapper_chain {
            normalize_wrapper_root(message);
        }
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
    // The value-level projections that no encoding can express: order and
    // duplicate loss of the map adapters, and the two cross-field
    // projections of the hand-written impls.
    match name.as_str() {
        "armonik.api.grpc.v1.Count" => normalize_count(message),
        "armonik.api.grpc.v1.tasks.GetResultIdsResponse" => {
            normalize_string_keyed_pairs(message, "task_results", "task_id");
        }
        "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse" => {
            normalize_string_keyed_pairs(message, "result_task", "result_id");
        }
        "armonik.api.grpc.v1.results.ImportResultsDataRequest" => {
            normalize_string_keyed_pairs(message, "results", "result_id");
        }
        "armonik.api.grpc.v1.results.ImportResultsDataResponse" => {
            normalize_string_keyed_pairs(message, "results", "name");
        }
        "armonik.api.grpc.v1.submitter.GetTaskStatusReply" => {
            normalize_string_keyed_pairs(message, "id_statuses", "task_id");
        }
        "armonik.api.grpc.v1.submitter.GetResultStatusReply" => {
            normalize_string_keyed_pairs(message, "id_statuses", "result_id");
        }
        // `success = true` wins over any error message.
        "armonik.api.grpc.v1.tasks.TaskDetailed.Output" => {
            let success = field(message, "success");
            if matches!(message.get_field(&success).as_ref(), Value::Bool(true)) {
                let error = field(message, "error");
                message.clear_field(&error);
            }
        }
        // The `ResultIdentifier` pairs are flattened into one shared session
        // ID (the first non-empty one) plus the result IDs.
        "armonik.api.grpc.v1.agent.NotifyResultDataRequest" => {
            normalize_notify_result_data(message);
        }
        _ => {}
    }
}

fn field(message: &DynamicMessage, name: &str) -> prost_reflect::FieldDescriptor {
    message
        .descriptor()
        .get_field_by_name(name)
        .unwrap_or_else(|| panic!("field `{name}` exists"))
}

/// Canonicalize an enum wrapper (chain) message: when the chained enum value
/// is zero, every representation (absent members, empty inner wrappers) is
/// equivalent to the empty message.
fn normalize_wrapper_root(message: &mut DynamicMessage) {
    let mut number = 0;
    let mut cursor = Value::Message(message.clone());
    loop {
        match cursor {
            Value::Message(wrapper) => {
                let Some(inner) = wrapper.descriptor().fields().next() else {
                    break;
                };
                cursor = wrapper.get_field(&inner).into_owned();
            }
            Value::EnumNumber(value) => {
                number = value;
                break;
            }
            _ => break,
        }
    }
    if number == 0 {
        let fields: Vec<_> = message.descriptor().fields().collect();
        for member in fields {
            message.clear_field(&member);
        }
    }
}

/// An absent `sort` message re-encodes: its always-emitted field member
/// keeps `Sort::default()` from being wire-empty. Oneof-typed field members
/// (`with_field`) additionally re-encode their default member.

/// Fold a repeated message member exposed as a `HashMap` keyed by one of the
/// entries' own string fields: duplicates collapse (last wins) and order is
/// lost, so entries are sorted by key.
fn normalize_string_keyed_pairs(message: &mut DynamicMessage, member: &str, key_name: &str) {
    let values = field(message, member);
    if !message.has_field(&values) {
        return;
    }
    let Value::List(entries) = message.get_field(&values).into_owned() else {
        return;
    };
    let mut by_key = std::collections::BTreeMap::new();
    for entry in entries {
        let Value::Message(pair) = &entry else {
            continue;
        };
        let key_field = field(pair, key_name);
        let key = match pair.get_field(&key_field).as_ref() {
            Value::String(key) => key.clone(),
            _ => String::new(),
        };
        by_key.insert(key, entry);
    }
    message.set_field(&values, Value::List(by_key.into_values().collect()));
}

/// Fold the repeated `StatusCount` pairs by status (last wins) and order
/// them, mirroring the `HashMap` representation.

fn normalize_notify_result_data(message: &mut DynamicMessage) {
    let ids = field(message, "ids");
    if !message.has_field(&ids) {
        return;
    }
    let Value::List(mut entries) = message.get_field(&ids).into_owned() else {
        return;
    };
    let session_id = entries
        .iter()
        .find_map(|entry| {
            let Value::Message(pair) = entry else {
                return None;
            };
            let session = field(pair, "session_id");
            match pair.get_field(&session).as_ref() {
                Value::String(session) if !session.is_empty() => Some(session.clone()),
                _ => None,
            }
        })
        .unwrap_or_default();
    for entry in &mut entries {
        let Value::Message(pair) = entry else {
            continue;
        };
        let session = field(pair, "session_id");
        pair.set_field(&session, Value::String(session_id.clone()));
    }
    message.set_field(&ids, Value::List(entries));
}

/// [`normalize_default_member`] for a message with several oneofs: fold an
/// absent member of the named oneof to the Rust default member.

/// Fold the repeated `StatusCount` pairs by status (last wins) and order
/// them, mirroring the `HashMap` representation.
fn normalize_count(message: &mut DynamicMessage) {
    let values = field(message, "values");
    if !message.has_field(&values) {
        return;
    }
    let Value::List(entries) = message.get_field(&values).into_owned() else {
        return;
    };
    let mut by_status = std::collections::BTreeMap::new();
    for entry in entries {
        let Value::Message(status_count) = &entry else {
            continue;
        };
        let status = field(status_count, "status");
        let key = match status_count.get_field(&status).as_ref() {
            Value::EnumNumber(number) => *number,
            _ => 0,
        };
        by_status.insert(key, entry);
    }
    message.set_field(&values, Value::List(by_status.into_values().collect()));
}
