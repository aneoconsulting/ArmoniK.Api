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
    "armonik.api.grpc.v1.sessions.SessionRaw" => armonik::sessions::Raw,
    "armonik.api.grpc.v1.sessions.SessionField" => armonik::sessions::Field,
    "armonik.api.grpc.v1.sessions.Filters" => armonik::sessions::filter::Or,
    "armonik.api.grpc.v1.sessions.FiltersAnd" => armonik::sessions::filter::And,
    "armonik.api.grpc.v1.sessions.FilterField" => armonik::sessions::filter::Field,
    "armonik.api.grpc.v1.sessions.ListSessionsRequest" => armonik::sessions::list::Request,
    "armonik.api.grpc.v1.sessions.ListSessionsRequest.Sort" => armonik::sessions::Sort,
    "armonik.api.grpc.v1.sessions.ListSessionsResponse" => armonik::sessions::list::Response,
    "armonik.api.grpc.v1.sessions.GetSessionRequest" => armonik::sessions::get::Request,
    "armonik.api.grpc.v1.sessions.GetSessionResponse" => armonik::sessions::get::Response,
    "armonik.api.grpc.v1.sessions.CancelSessionRequest" => armonik::sessions::cancel::Request,
    "armonik.api.grpc.v1.sessions.CancelSessionResponse" => armonik::sessions::cancel::Response,
    "armonik.api.grpc.v1.sessions.CreateSessionRequest" => armonik::sessions::create::Request,
    "armonik.api.grpc.v1.sessions.CreateSessionReply" => armonik::sessions::create::Response,
    "armonik.api.grpc.v1.sessions.PauseSessionRequest" => armonik::sessions::pause::Request,
    "armonik.api.grpc.v1.sessions.PauseSessionResponse" => armonik::sessions::pause::Response,
    "armonik.api.grpc.v1.sessions.ResumeSessionRequest" => armonik::sessions::resume::Request,
    "armonik.api.grpc.v1.sessions.ResumeSessionResponse" => armonik::sessions::resume::Response,
    "armonik.api.grpc.v1.sessions.CloseSessionRequest" => armonik::sessions::close::Request,
    "armonik.api.grpc.v1.sessions.CloseSessionResponse" => armonik::sessions::close::Response,
    "armonik.api.grpc.v1.sessions.PurgeSessionRequest" => armonik::sessions::purge::Request,
    "armonik.api.grpc.v1.sessions.PurgeSessionResponse" => armonik::sessions::purge::Response,
    "armonik.api.grpc.v1.sessions.DeleteSessionRequest" => armonik::sessions::delete::Request,
    "armonik.api.grpc.v1.sessions.DeleteSessionResponse" => armonik::sessions::delete::Response,
    "armonik.api.grpc.v1.sessions.StopSubmissionRequest"
        => armonik::sessions::stop_submission::Request,
    "armonik.api.grpc.v1.sessions.StopSubmissionResponse"
        => armonik::sessions::stop_submission::Response,
    "armonik.api.grpc.v1.tasks.TaskDetailed" => armonik::tasks::Raw,
    "armonik.api.grpc.v1.tasks.TaskDetailed.Output" => armonik::tasks::Output,
    "armonik.api.grpc.v1.tasks.TaskSummary" => armonik::tasks::Summary,
    "armonik.api.grpc.v1.tasks.TaskField" => armonik::tasks::Field,
    "armonik.api.grpc.v1.tasks.Filters" => armonik::tasks::filter::Or,
    "armonik.api.grpc.v1.tasks.FiltersAnd" => armonik::tasks::filter::And,
    "armonik.api.grpc.v1.tasks.FilterField" => armonik::tasks::filter::Field,
    "armonik.api.grpc.v1.tasks.ListTasksRequest" => armonik::tasks::list::Request,
    "armonik.api.grpc.v1.tasks.ListTasksRequest.Sort" => armonik::tasks::Sort,
    "armonik.api.grpc.v1.tasks.ListTasksResponse" => armonik::tasks::list::Response,
    "armonik.api.grpc.v1.tasks.ListTasksDetailedResponse" => armonik::tasks::list_detailed::Response,
    "armonik.api.grpc.v1.tasks.GetTaskRequest" => armonik::tasks::get::Request,
    "armonik.api.grpc.v1.tasks.GetTaskResponse" => armonik::tasks::get::Response,
    "armonik.api.grpc.v1.tasks.CancelTasksRequest" => armonik::tasks::cancel::Request,
    "armonik.api.grpc.v1.tasks.CancelTasksResponse" => armonik::tasks::cancel::Response,
    "armonik.api.grpc.v1.tasks.GetResultIdsRequest" => armonik::tasks::get_result_ids::Request,
    "armonik.api.grpc.v1.tasks.GetResultIdsResponse" => armonik::tasks::get_result_ids::Response,
    "armonik.api.grpc.v1.tasks.CountTasksByStatusRequest" => armonik::tasks::count_status::Request,
    "armonik.api.grpc.v1.tasks.CountTasksByStatusResponse"
        => armonik::tasks::count_status::Response,
    "armonik.api.grpc.v1.tasks.SubmitTasksRequest" => armonik::tasks::submit::Request,
    "armonik.api.grpc.v1.tasks.SubmitTasksRequest.TaskCreation"
        => armonik::tasks::submit::RequestItem,
    "armonik.api.grpc.v1.tasks.SubmitTasksResponse" => armonik::tasks::submit::Response,
    "armonik.api.grpc.v1.tasks.SubmitTasksResponse.TaskInfo"
        => armonik::tasks::submit::ResponseItem,
    "armonik.api.grpc.v1.results.ResultRaw" => armonik::results::Raw,
    "armonik.api.grpc.v1.results.Filters" => armonik::results::filter::Or,
    "armonik.api.grpc.v1.results.FiltersAnd" => armonik::results::filter::And,
    "armonik.api.grpc.v1.results.FilterField" => armonik::results::filter::Field,
    "armonik.api.grpc.v1.results.ListResultsRequest" => armonik::results::list::Request,
    "armonik.api.grpc.v1.results.ListResultsRequest.Sort" => armonik::results::Sort,
    "armonik.api.grpc.v1.results.ListResultsResponse" => armonik::results::list::Response,
    "armonik.api.grpc.v1.results.GetResultRequest" => armonik::results::get::Request,
    "armonik.api.grpc.v1.results.GetResultResponse" => armonik::results::get::Response,
    "armonik.api.grpc.v1.results.GetOwnerTaskIdRequest"
        => armonik::results::get_owner_task_id::Request,
    "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse"
        => armonik::results::get_owner_task_id::Response,
    "armonik.api.grpc.v1.results.CreateResultsMetaDataRequest"
        => armonik::results::create_metadata::Request,
    "armonik.api.grpc.v1.results.CreateResultsMetaDataRequest.ResultCreate"
        => armonik::results::create_metadata::RequestItem,
    "armonik.api.grpc.v1.results.CreateResultsMetaDataResponse"
        => armonik::results::create_metadata::Response,
    "armonik.api.grpc.v1.results.CreateResultsRequest" => armonik::results::create::Request,
    "armonik.api.grpc.v1.results.CreateResultsRequest.ResultCreate"
        => armonik::results::create::RequestItem,
    "armonik.api.grpc.v1.results.CreateResultsResponse" => armonik::results::create::Response,
    "armonik.api.grpc.v1.results.ImportResultsDataRequest" => armonik::results::import::Request,
    "armonik.api.grpc.v1.results.ImportResultsDataResponse" => armonik::results::import::Response,
    "armonik.api.grpc.v1.results.DeleteResultsDataRequest"
        => armonik::results::delete_data::Request,
    "armonik.api.grpc.v1.results.DeleteResultsDataResponse"
        => armonik::results::delete_data::Response,
    "armonik.api.grpc.v1.results.UploadResultDataRequest" => armonik::results::upload::Request,
    "armonik.api.grpc.v1.results.UploadResultDataResponse" => armonik::results::upload::Response,
    "armonik.api.grpc.v1.results.DownloadResultDataRequest"
        => armonik::results::download::Request,
    "armonik.api.grpc.v1.results.DownloadResultDataResponse"
        => armonik::results::download::Response,
    "armonik.api.grpc.v1.results.ResultsServiceConfigurationResponse"
        => armonik::results::get_service_configuration::Response,
    "armonik.api.grpc.v1.events.EventSubscriptionRequest" => armonik::events::subscribe::Request,
    "armonik.api.grpc.v1.events.EventSubscriptionResponse"
        => armonik::events::subscribe::Response,
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.TaskStatusUpdate"
        => armonik::events::TaskStatusUpdate,
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultStatusUpdate"
        => armonik::events::ResultStatusUpdate,
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultOwnerUpdate"
        => armonik::events::ResultOwnerUpdate,
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.NewTask" => armonik::events::NewTask,
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.NewResult" => armonik::events::NewResult,
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
        | "armonik.api.grpc.v1.partitions.PartitionField"
        | "armonik.api.grpc.v1.results.ResultField" => {
            normalize_wrapper_root(message);
        }
        // Filter fields: the condition oneof defaults to an empty string
        // filter, and enum wrappers fold zero/absent/empty uniformly.
        "armonik.api.grpc.v1.applications.FilterField"
        | "armonik.api.grpc.v1.partitions.FilterField"
        | "armonik.api.grpc.v1.sessions.FilterField" => {
            normalize_default_member(message, "filter_string");
            // Absent/empty wrappers fold to the API default (Name/Id = 1),
            // like the historical `map_or_else(Default::default, ...)`.
            normalize_enum_wrapper(message, "field", 1);
        }
        "armonik.api.grpc.v1.sessions.ListSessionsRequest" => {
            normalize_default_sort(message, side, Some(1));
        }
        "armonik.api.grpc.v1.sessions.ListSessionsRequest.Sort" => {
            normalize_enum_wrapper(message, "field", 1);
        }
        // A memberless field oneof decodes to the API default (SessionId).
        "armonik.api.grpc.v1.sessions.SessionField" => {
            if !any_member_set(message) {
                normalize_enum_wrapper(message, "session_raw_field", 1);
            }
        }
        "armonik.api.grpc.v1.sessions.SessionRawField"
        | "armonik.api.grpc.v1.sessions.TaskOptionField"
        | "armonik.api.grpc.v1.tasks.TaskOptionField" => {
            normalize_wrapper_root(message);
        }
        // Task-options members kept as a plain `TaskOptions`.
        "armonik.api.grpc.v1.sessions.SessionRaw" => {
            normalize_task_options_member(message, side, "options");
        }
        "armonik.api.grpc.v1.sessions.CreateSessionRequest" => {
            normalize_task_options_member(message, side, "default_task_option");
        }
        "armonik.api.grpc.v1.tasks.ListTasksRequest" => {
            normalize_default_sort(message, side, Some(16));
        }
        "armonik.api.grpc.v1.tasks.ListTasksRequest.Sort" => {
            normalize_enum_wrapper(message, "field", 16);
        }
        // A memberless field oneof decodes to the API default (TaskId).
        "armonik.api.grpc.v1.tasks.TaskField" => {
            if !any_member_set(message) {
                normalize_enum_wrapper(message, "task_summary_field", 16);
            }
        }
        "armonik.api.grpc.v1.tasks.TaskSummaryField" => {
            normalize_wrapper_root(message);
        }
        "armonik.api.grpc.v1.tasks.FilterField" => {
            normalize_default_member(message, "filter_string");
            normalize_enum_wrapper(message, "field", 16);
        }
        // `success = true` wins over any error message.
        "armonik.api.grpc.v1.tasks.TaskDetailed.Output" => {
            let success = field(message, "success");
            if matches!(message.get_field(&success).as_ref(), Value::Bool(true)) {
                let error = field(message, "error");
                message.clear_field(&error);
            }
        }
        "armonik.api.grpc.v1.tasks.TaskDetailed" => {
            normalize_task_options_member(message, side, "options");
            normalize_output_member(message, side);
        }
        "armonik.api.grpc.v1.tasks.TaskSummary" => {
            normalize_task_options_member(message, side, "options");
        }
        // Detailed-task member kept as a plain `tasks::Raw`.
        "armonik.api.grpc.v1.tasks.GetTaskResponse" => {
            normalize_task_member(message, side);
        }
        // Repeated pairs exposed as a map: order is lost and duplicate
        // keys collapse (last wins).
        "armonik.api.grpc.v1.tasks.GetResultIdsResponse" => {
            normalize_string_keyed_pairs(message, "task_results", "task_id");
        }
        "armonik.api.grpc.v1.results.ListResultsRequest" => {
            normalize_default_sort(message, side, Some(7));
        }
        "armonik.api.grpc.v1.results.ListResultsRequest.Sort" => {
            normalize_enum_wrapper(message, "field", 7);
        }
        "armonik.api.grpc.v1.results.FilterField" => {
            normalize_default_member(message, "filter_string");
            normalize_enum_wrapper(message, "field", 7);
        }
        "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse" => {
            normalize_string_keyed_pairs(message, "result_task", "result_id");
        }
        "armonik.api.grpc.v1.results.ImportResultsDataRequest" => {
            normalize_string_keyed_pairs(message, "results", "result_id");
        }
        // Repeated results exposed as a map keyed by their own name.
        "armonik.api.grpc.v1.results.ImportResultsDataResponse" => {
            normalize_string_keyed_pairs(message, "results", "name");
        }
        "armonik.api.grpc.v1.results.UploadResultDataRequest" => {
            normalize_default_member(message, "id");
        }
        // Raw-session members kept as a plain `sessions::Raw`.
        "armonik.api.grpc.v1.sessions.GetSessionResponse"
        | "armonik.api.grpc.v1.sessions.CancelSessionResponse"
        | "armonik.api.grpc.v1.sessions.PauseSessionResponse"
        | "armonik.api.grpc.v1.sessions.ResumeSessionResponse"
        | "armonik.api.grpc.v1.sessions.CloseSessionResponse"
        | "armonik.api.grpc.v1.sessions.PurgeSessionResponse"
        | "armonik.api.grpc.v1.sessions.DeleteSessionResponse"
        | "armonik.api.grpc.v1.sessions.StopSubmissionResponse" => {
            normalize_session_member(message, side);
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

/// A task-options member kept as a plain `TaskOptions`: an absent member
/// folds to the API default on the original side (the historical
/// `unwrap_or_default`), and to the wire-zero form on the way back (a
/// wire-zero `TaskOptions` encodes empty and is dropped); both foldings are
/// then max_duration-canonicalized like any present `TaskOptions`.
fn normalize_task_options_member(message: &mut DynamicMessage, side: Side, member: &str) {
    let member = field(message, member);
    if message.has_field(&member) {
        return;
    }
    let prost_reflect::Kind::Message(desc) = member.kind() else {
        panic!("task-options member is a message");
    };
    let mut options = DynamicMessage::new(desc);
    if side == Side::Original {
        options.set_field_by_name("max_retries", Value::I32(1));
        options.set_field_by_name("priority", Value::I32(1));
    }
    normalize_task_options(&mut options);
    message.set_field(&member, Value::Message(options));
}

/// A raw-session member kept as a plain `sessions::Raw`: absent members fold
/// like [`normalize_task_options_member`], one level deeper.
fn normalize_session_member(message: &mut DynamicMessage, side: Side) {
    let member = field(message, "session");
    if message.has_field(&member) {
        return;
    }
    let prost_reflect::Kind::Message(desc) = member.kind() else {
        panic!("session member is a message");
    };
    let mut session = DynamicMessage::new(desc);
    normalize_task_options_member(&mut session, side, "options");
    message.set_field(&member, Value::Message(session));
}

/// An output member kept as a plain `tasks::Output`: an absent output means
/// success, and the wire form always carries the output (an empty error
/// encodes as an empty message), so absence only folds on the original side.
fn normalize_output_member(message: &mut DynamicMessage, side: Side) {
    let _ = side;
    let member = field(message, "output");
    if message.has_field(&member) {
        return;
    }
    let prost_reflect::Kind::Message(desc) = member.kind() else {
        panic!("output member is a message");
    };
    let mut output = DynamicMessage::new(desc);
    output.set_field_by_name("success", Value::Bool(true));
    message.set_field(&member, Value::Message(output));
}

/// A detailed-task member kept as a plain `tasks::Raw`: absent members fold
/// like [`normalize_task_options_member`], one level deeper.
fn normalize_task_member(message: &mut DynamicMessage, side: Side) {
    let member = field(message, "task");
    if message.has_field(&member) {
        return;
    }
    let prost_reflect::Kind::Message(desc) = member.kind() else {
        panic!("task member is a message");
    };
    let mut task = DynamicMessage::new(desc);
    normalize_task_options_member(&mut task, side, "options");
    normalize_output_member(&mut task, side);
    message.set_field(&member, Value::Message(task));
}

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
