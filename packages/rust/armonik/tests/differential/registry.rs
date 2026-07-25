//! Mapping from proto full names to the armonik types implementing them,
//! and the projection of messages onto the armonik types' documented
//! equivalence classes.
//!
//! Grown in lockstep with the annotation of `src/objects/`: registering a
//! type here removes it from `TEMP_UNMAPPED` in `main.rs` (the coverage
//! test enforces both directions).

use armonik::reexports::prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage, Value};

pub struct Entry {
    pub proto: &'static str,
    /// Decode the bytes as the armonik type and re-encode them.
    pub roundtrip: fn(&[u8]) -> Result<Vec<u8>, armonik::reexports::prost::DecodeError>,
}

macro_rules! registry {
    ($($proto:literal => $ty:ty),* $(,)?) => {
        pub fn entries() -> Vec<Entry> {
            vec![$(Entry {
                proto: $proto,
                roundtrip: |bytes| Ok(<$ty as Message>::decode(bytes)?.encode_to_vec()),
            }),*]
        }
    };
}

registry! {
    "armonik.api.grpc.v1.Configuration" => armonik::Configuration,
    "armonik.api.grpc.v1.Count" => armonik::Count,
    "armonik.api.grpc.v1.Error" => armonik::Error,
    "armonik.api.grpc.v1.FilterArray" => armonik::FilterArray,
    "armonik.api.grpc.v1.FilterBoolean" => armonik::FilterBoolean,
    "armonik.api.grpc.v1.FilterDate" => armonik::FilterDate,
    "armonik.api.grpc.v1.FilterDuration" => armonik::FilterDuration,
    "armonik.api.grpc.v1.FilterNumber" => armonik::FilterNumber,
    "armonik.api.grpc.v1.FilterString" => armonik::FilterString,
    "armonik.api.grpc.v1.sessions.FilterStatus"
        => armonik::FilterStatus<armonik::SessionStatus>,
    "armonik.api.grpc.v1.tasks.FilterStatus"
        => armonik::FilterStatus<armonik::TaskStatus>,
    "armonik.api.grpc.v1.results.FilterStatus"
        => armonik::FilterStatus<armonik::ResultStatus>,
    "armonik.api.grpc.v1.ResultRequest" => armonik::ResultRequest,
    "armonik.api.grpc.v1.TaskError" => armonik::TaskError,
    "armonik.api.grpc.v1.TaskId" => armonik::TaskId,
    "armonik.api.grpc.v1.TaskIdList" => armonik::TaskIdList,
    "armonik.api.grpc.v1.TaskIdWithStatus" => armonik::TaskIdWithStatus,
    "armonik.api.grpc.v1.TaskList" => armonik::TaskList,
    "armonik.api.grpc.v1.TaskOutputRequest" => armonik::TaskOutputRequest,
    "armonik.api.grpc.v1.TaskRequest" => armonik::TaskRequest,
    "armonik.api.grpc.v1.DataChunk" => armonik::DataChunk,
    "armonik.api.grpc.v1.InitKeyedDataStream" => armonik::InitKeyedDataStream,
    "armonik.api.grpc.v1.InitTaskRequest" => armonik::InitTaskRequest,
    "armonik.api.grpc.v1.Output" => armonik::Output,
    "armonik.api.grpc.v1.Session" => armonik::Session,
    "armonik.api.grpc.v1.StatusCount" => armonik::StatusCount,
    "armonik.api.grpc.v1.TaskOptions" => armonik::TaskOptions,
    "armonik.api.grpc.v1.TaskRequestHeader" => armonik::TaskRequestHeader,
    "armonik.api.grpc.v1.agent.CreateTaskRequest" => armonik::agent::create_tasks::Request,
    "armonik.api.grpc.v1.agent.CreateTaskRequest.InitRequest"
        => armonik::agent::create_tasks::InitRequest,
    "armonik.api.grpc.v1.agent.CreateTaskReply" => armonik::agent::create_tasks::Response,
    "armonik.api.grpc.v1.agent.CreateTaskReply.CreationStatus"
        => armonik::agent::create_tasks::Status,
    "armonik.api.grpc.v1.applications.ApplicationRaw" => armonik::applications::Raw,
    "armonik.api.grpc.v1.applications.Filters" => armonik::applications::filter::Or,
    "armonik.api.grpc.v1.applications.FiltersAnd" => armonik::applications::filter::And,
    "armonik.api.grpc.v1.applications.FilterField" => armonik::applications::filter::Field,
    "armonik.api.grpc.v1.applications.ListApplicationsRequest"
        => armonik::applications::list::Request,
    "armonik.api.grpc.v1.applications.ListApplicationsRequest.Sort"
        => armonik::applications::Sort,
    "armonik.api.grpc.v1.applications.ListApplicationsResponse"
        => armonik::applications::list::Response,
    "armonik.api.grpc.v1.partitions.PartitionRaw" => armonik::partitions::Raw,
    "armonik.api.grpc.v1.partitions.Filters" => armonik::partitions::filter::Or,
    "armonik.api.grpc.v1.partitions.FiltersAnd" => armonik::partitions::filter::And,
    "armonik.api.grpc.v1.partitions.FilterField" => armonik::partitions::filter::Field,
    "armonik.api.grpc.v1.partitions.GetPartitionRequest" => armonik::partitions::get::Request,
    "armonik.api.grpc.v1.partitions.GetPartitionResponse" => armonik::partitions::get::Response,
    "armonik.api.grpc.v1.partitions.ListPartitionsRequest" => armonik::partitions::list::Request,
    "armonik.api.grpc.v1.partitions.ListPartitionsRequest.Sort" => armonik::partitions::Sort,
    "armonik.api.grpc.v1.partitions.ListPartitionsResponse"
        => armonik::partitions::list::Response,
    "armonik.api.grpc.v1.auth.GetCurrentUserRequest" => armonik::auth::current_user::Request,
    "armonik.api.grpc.v1.auth.GetCurrentUserResponse" => armonik::auth::current_user::Response,
    "armonik.api.grpc.v1.auth.User" => armonik::auth::User,
    "armonik.api.grpc.v1.health_checks.CheckHealthRequest"
        => armonik::health_checks::check::Request,
    "armonik.api.grpc.v1.health_checks.CheckHealthResponse"
        => armonik::health_checks::check::Response,
    "armonik.api.grpc.v1.health_checks.CheckHealthResponse.ServiceHealth"
        => armonik::health_checks::ServiceHealth,
    "armonik.api.grpc.v1.versions.ListVersionsRequest" => armonik::versions::list::Request,
    "armonik.api.grpc.v1.versions.ListVersionsResponse" => armonik::versions::list::Response,
}

/// Project a message (recursively) onto the equivalence classes of its
/// armonik type, so that the semantic comparison reflects the documented
/// semantics. Applied to both sides of every round-trip.
/// Which side of the round-trip is being normalized: some folds (e.g.
/// absent sort => API default) only apply to the generated original, because
/// the round-trip itself resolves the ambiguity on the way back.
#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Original,
    Back,
}

pub fn normalize(message: &mut DynamicMessage, side: Side) {
    let descriptor = message.descriptor();
    for field in descriptor.fields() {
        if !message.has_field(&field) {
            continue;
        }
        let mut value = message.get_field(&field).into_owned();
        if normalize_value(&mut value, side) {
            message.set_field(&field, value);
        }
    }
    apply_rules(message, side);
}

fn normalize_value(value: &mut Value, side: Side) -> bool {
    match value {
        Value::Message(inner) => {
            normalize(inner, side);
            true
        }
        Value::List(items) => {
            let mut changed = false;
            for item in items {
                changed |= normalize_value(item, side);
            }
            changed
        }
        Value::Map(map) => {
            let mut changed = false;
            for item in map.values_mut() {
                changed |= normalize_value(item, side);
            }
            changed
        }
        _ => false,
    }
}

fn apply_rules(message: &mut DynamicMessage, side: Side) {
    match message.descriptor().full_name() {
        // Absent or empty max_duration folds to the INFINITE_DURATION
        // default of `TaskOptions`.
        "armonik.api.grpc.v1.TaskOptions" => normalize_task_options(message),
        // Marker members only remember which member was set; oneofs whose
        // Rust `Default` is a member variant re-encode an absent oneof with
        // that member present — like the historical None => Default.
        "armonik.api.grpc.v1.DataChunk" => {
            normalize_bool_marker(message, "data_complete");
            normalize_default_member(message, "data");
        }
        "armonik.api.grpc.v1.InitKeyedDataStream" => {
            normalize_bool_marker(message, "last_result");
            normalize_default_member(message, "key");
        }
        "armonik.api.grpc.v1.InitTaskRequest" => {
            normalize_bool_marker(message, "last_task");
            normalize_default_member(message, "header");
        }
        "armonik.api.grpc.v1.Output" => normalize_default_member(message, "ok"),
        // Repeated pairs exposed as a map: order is lost and duplicate
        // statuses collapse (last wins).
        "armonik.api.grpc.v1.Count" => normalize_count(message),
        // The historical conversion drops the token when no member is set
        // (`Request::Invalid` has no slot for it).
        "armonik.api.grpc.v1.agent.CreateTaskRequest" => {
            if !any_member_set(message) {
                let token = field(message, "communication_token");
                message.clear_field(&token);
            }
        }
        // `Response`'s default is an empty `Error` carrying the token.
        "armonik.api.grpc.v1.agent.CreateTaskReply" => {
            normalize_default_member(message, "error");
        }
        "armonik.api.grpc.v1.agent.CreateTaskReply.CreationStatus" => {
            normalize_default_member(message, "error");
        }
        // List requests: an absent/empty sort folds to the API default,
        // whose direction is ascending (1).
        "armonik.api.grpc.v1.applications.ListApplicationsRequest" => {
            normalize_default_sort(message, side, None);
        }
        "armonik.api.grpc.v1.partitions.ListPartitionsRequest" => {
            normalize_default_sort(message, side, Some(1));
        }
        // Standalone sorts: an absent field member folds to the API default.
        "armonik.api.grpc.v1.partitions.ListPartitionsRequest.Sort" => {
            normalize_enum_wrapper(message, "field", 1);
        }
        // Wrapper chains: zero, absent and present-but-empty carry no
        // information; canonicalize to the empty wrapper.
        "armonik.api.grpc.v1.applications.ApplicationField"
        | "armonik.api.grpc.v1.partitions.PartitionField" => {
            normalize_wrapper_root(message);
        }
        // Filter fields: the condition oneof defaults to an empty string
        // filter, and enum wrappers fold zero/absent/empty uniformly.
        "armonik.api.grpc.v1.applications.FilterField"
        | "armonik.api.grpc.v1.partitions.FilterField" => {
            normalize_default_member(message, "filter_string");
            // Absent/empty wrappers fold to the API default (Name/Id = 1),
            // like the historical `map_or_else(Default::default, ...)`.
            normalize_enum_wrapper(message, "field", 1);
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

fn any_member_set(message: &DynamicMessage) -> bool {
    let descriptor = message.descriptor();
    let oneof = descriptor.oneofs().next().expect("flattened oneof exists");
    let member_set = oneof.fields().any(|member| message.has_field(&member));
    member_set
}

fn normalize_bool_marker(message: &mut DynamicMessage, member: &str) {
    let member = field(message, member);
    if message.has_field(&member) {
        message.set_field(&member, Value::Bool(true));
    }
}

fn normalize_default_member(message: &mut DynamicMessage, member: &str) {
    if !any_member_set(message) {
        let member = field(message, member);
        message.set_field(&member, Value::default_value_for_field(&member));
    }
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

/// An absent or empty `sort` message folds to the API default, whose
/// direction is ascending; like the historical `unwrap_or_default`.
fn normalize_default_sort(message: &mut DynamicMessage, side: Side, field_default: Option<i32>) {
    let sort = field(message, "sort");
    // A fully absent sort folds to the API default (ascending direction) on
    // the original side only: an absent sort on the way back stems from a
    // present-but-empty one, which the wire form legitimately drops.
    if side == Side::Original && !message.has_field(&sort) {
        let prost_reflect::Kind::Message(desc) = sort.kind() else {
            panic!("sort is a message");
        };
        let mut default_sort = DynamicMessage::new(desc);
        default_sort.set_field_by_name("direction", Value::EnumNumber(1));
        message.set_field(&sort, Value::Message(default_sort));
    }
    // Within a (possibly folded) sort, an absent field member folds to the
    // API default field, like the nested historical `unwrap_or_default`.
    if let Some(number) = field_default {
        let Value::Message(mut inner) = message.get_field(&sort).into_owned() else {
            return;
        };
        normalize_enum_wrapper(&mut inner, "field", number);
        message.set_field(&sort, Value::Message(inner));
    }
}

/// Enum wrappers (possibly chained) fold "absent", "empty" and "zero" into
/// the API default of the flattened enum: project the member field onto the
/// wrapper chain carrying `default_number` when it holds no information.
fn normalize_enum_wrapper(message: &mut DynamicMessage, member: &str, default_number: i32) {
    let member = field(message, member);
    // Only a truly absent member folds to the API default; a present wrapper
    // (even empty, i.e. explicit zero) is preserved by the wire form.
    if message.has_field(&member) {
        return;
    }
    // Build the wrapper chain down to the enum field.
    let prost_reflect::Kind::Message(mut desc) = member.kind() else {
        panic!("enum wrapper member is a message");
    };
    let mut chain = Vec::new();
    let enum_field = loop {
        let inner = desc.fields().next().expect("wrapper has one field");
        match inner.kind() {
            prost_reflect::Kind::Message(next) => {
                chain.push((desc.clone(), inner));
                desc = next;
            }
            prost_reflect::Kind::Enum(_) => break inner,
            other => panic!("unexpected wrapper field kind {other:?}"),
        }
    };
    let mut value = DynamicMessage::new(desc);
    value.set_field(&enum_field, Value::EnumNumber(default_number));
    let mut wrapped = value;
    for (outer_desc, outer_field) in chain.into_iter().rev() {
        let mut outer = DynamicMessage::new(outer_desc);
        outer.set_field(&outer_field, Value::Message(wrapped));
        wrapped = outer;
    }
    message.set_field(&member, Value::Message(wrapped));
}

fn normalize_task_options(message: &mut DynamicMessage) {
    let max_duration = field(message, "max_duration");
    let is_empty = match message.get_field(&max_duration).as_ref() {
        Value::Message(duration) => duration.encode_to_vec().is_empty(),
        _ => true,
    };
    if is_empty {
        let duration_desc = match max_duration.kind() {
            prost_reflect::Kind::Message(desc) => desc,
            other => panic!("max_duration should be a message, got {other:?}"),
        };
        let mut infinite = DynamicMessage::new(duration_desc);
        infinite.set_field_by_name("seconds", Value::I64(315576000000));
        message.set_field(&max_duration, Value::Message(infinite));
    }
}

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
