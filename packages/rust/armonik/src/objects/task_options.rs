use ::std::collections::HashMap;

use crate::api::v3;

const INFINITE_DURATION: prost_types::Duration = prost_types::Duration {
    seconds: 315576000000,
    nanos: 0,
};

#[derive(Debug, Clone, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskOptions")]
pub struct TaskOptions {
    pub options: HashMap<String, String>,
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_duration"))]
    pub max_duration: prost_types::Duration,
    pub max_retries: i32,
    pub priority: i32,
    pub partition_id: String,
    pub application_name: String,
    pub application_version: String,
    pub application_namespace: String,
    pub application_service: String,
    pub engine_type: String,
}

impl std::cmp::PartialEq for TaskOptions {
    fn eq(&self, other: &Self) -> bool {
        self.max_duration.seconds == other.max_duration.seconds
            && self.max_duration.nanos == other.max_duration.nanos
            && self.max_retries == other.max_retries
            && self.priority == other.priority
            && self.partition_id == other.partition_id
            && self.application_name == other.application_name
            && self.application_version == other.application_version
            && self.application_namespace == other.application_namespace
            && self.application_service == other.application_service
            && self.engine_type == other.engine_type
    }
}

impl std::cmp::Eq for TaskOptions {}

impl Default for TaskOptions {
    fn default() -> Self {
        Self {
            options: Default::default(),
            max_duration: INFINITE_DURATION,
            max_retries: 1,
            priority: 1,
            partition_id: Default::default(),
            application_name: Default::default(),
            application_version: Default::default(),
            application_namespace: Default::default(),
            application_service: Default::default(),
            engine_type: Default::default(),
        }
    }
}

/// Represents a field in a task option.
///
/// Stands for the single-enum-field wrapper messages
/// `sessions.TaskOptionField` and `tasks.TaskOptionField`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent)]
#[armonik(message = "armonik.api.grpc.v1.sessions.TaskOptionField")]
#[armonik(message = "armonik.api.grpc.v1.tasks.TaskOptionField")]
pub enum TaskOptionField {
    MaxDuration,
    MaxRetries,
    Priority,
    PartitionId,
    ApplicationName,
    ApplicationVersion,
    ApplicationNamespace,
    ApplicationService,
    /// Named `ENGINE_TYPE` in the protos.
    #[armonik(rename = "TASK_OPTION_ENUM_FIELD_ENGINE_TYPE")]
    ApplicationEngine,
    /// Unspecified (zero) or a field unknown to this crate version.
    Other(OtherTaskOptionField),
}

impl From<TaskOptionField> for v3::sessions::TaskOptionField {
    fn from(value: TaskOptionField) -> Self {
        Self {
            field: i32::from(value),
        }
    }
}

impl From<TaskOptionField> for v3::tasks::TaskOptionField {
    fn from(value: TaskOptionField) -> Self {
        Self {
            field: i32::from(value),
        }
    }
}

impl From<v3::sessions::TaskOptionField> for TaskOptionField {
    fn from(value: v3::sessions::TaskOptionField) -> Self {
        value.field.into()
    }
}

impl From<v3::tasks::TaskOptionField> for TaskOptionField {
    fn from(value: v3::tasks::TaskOptionField) -> Self {
        value.field.into()
    }
}

super::super::impl_convert!(req TaskOptionField : v3::sessions::TaskOptionField);
super::super::impl_convert!(req TaskOptionField : v3::tasks::TaskOptionField);

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use prost::Message;

    use super::{TaskOptions, INFINITE_DURATION};

    /// prost-derived reference (the generated type no longer exists).
    #[derive(Clone, PartialEq, Message)]
    struct RefOptions {
        #[prost(map = "string, string", tag = "1")]
        options: HashMap<String, String>,
        #[prost(message, optional, tag = "2")]
        max_duration: Option<prost_types::Duration>,
        #[prost(int32, tag = "3")]
        max_retries: i32,
        #[prost(int32, tag = "4")]
        priority: i32,
        #[prost(string, tag = "5")]
        partition_id: String,
        #[prost(string, tag = "6")]
        application_name: String,
        #[prost(string, tag = "7")]
        application_version: String,
        #[prost(string, tag = "8")]
        application_namespace: String,
        #[prost(string, tag = "9")]
        application_service: String,
        #[prost(string, tag = "10")]
        engine_type: String,
    }

    /// The derived implementation must round-trip against the reference.
    #[test]
    fn derived_message_roundtrips() {
        let ours = TaskOptions {
            options: [("k".to_owned(), "v".to_owned())].into_iter().collect(),
            max_duration: prost_types::Duration {
                seconds: 60,
                nanos: 7,
            },
            max_retries: 4,
            priority: 2,
            partition_id: "part".into(),
            application_name: "app".into(),
            application_version: "1.0".into(),
            application_namespace: "ns".into(),
            application_service: "svc".into(),
            engine_type: "engine".into(),
        };
        let theirs = RefOptions::decode(ours.encode_to_vec().as_slice()).unwrap();
        assert_eq!(theirs.options, ours.options);
        assert_eq!(theirs.max_duration, Some(ours.max_duration));
        assert_eq!(theirs.max_retries, ours.max_retries);
        assert_eq!(theirs.priority, ours.priority);
        assert_eq!(theirs.partition_id, ours.partition_id);

        let back = TaskOptions::decode(theirs.encode_to_vec().as_slice()).unwrap();
        assert_eq!(back, ours);
        assert_eq!(back.options, ours.options);
    }

    /// Absent max_duration decodes to INFINITE_DURATION (the seed), exactly
    /// like the historical unwrap_or(INFINITE_DURATION) conversion; and it
    /// re-encodes explicitly since it is not the proto zero value.
    #[test]
    fn custom_default_survives_via_merge_seeding() {
        let absent = RefOptions {
            max_duration: None,
            max_retries: 3,
            ..Default::default()
        };
        let ours = TaskOptions::decode(absent.encode_to_vec().as_slice()).unwrap();
        assert_eq!(ours.max_duration.seconds, INFINITE_DURATION.seconds);
        assert_eq!(ours.max_duration.nanos, INFINITE_DURATION.nanos);
        // Absent scalars decode to the proto zero, not the API default.
        assert_eq!(ours.priority, 0);

        let reencoded = RefOptions::decode(ours.encode_to_vec().as_slice()).unwrap();
        assert_eq!(reencoded.max_duration, Some(INFINITE_DURATION));
    }

    /// A wire occurrence of `max_duration` must merge from the proto zero
    /// value, not from the `INFINITE_DURATION` seed.
    #[test]
    fn present_duration_does_not_inherit_the_seed() {
        let partial = RefOptions {
            max_duration: Some(prost_types::Duration {
                seconds: 0,
                nanos: 7,
            }),
            ..Default::default()
        };
        let ours = TaskOptions::decode(partial.encode_to_vec().as_slice()).unwrap();
        assert_eq!(ours.max_duration.seconds, 0);
        assert_eq!(ours.max_duration.nanos, 7);

        let explicit_zero = RefOptions {
            max_duration: Some(prost_types::Duration::default()),
            ..Default::default()
        };
        let ours = TaskOptions::decode(explicit_zero.encode_to_vec().as_slice()).unwrap();
        assert_eq!(ours.max_duration, prost_types::Duration::default());
    }

    /// `TaskOptionField` stands for the single-enum-field wrapper messages;
    /// its wire form must match the generated wrappers of both services.
    #[test]
    fn transparent_wrapper_roundtrips() {
        use prost::encoding::{decode_key, DecodeContext};

        use super::TaskOptionField;
        use crate::codec::ProtoField;

        #[derive(Clone, PartialEq, prost::Message)]
        struct Ref {
            #[prost(message, optional, tag = "1")]
            field: Option<crate::api::v3::sessions::TaskOptionField>,
        }

        // The old ApplicationEngine variant maps to the proto ENGINE_TYPE.
        assert_eq!(i32::from(TaskOptionField::ApplicationEngine), 9);
        assert_eq!(TaskOptionField::from(9), TaskOptionField::ApplicationEngine);

        for value in [
            TaskOptionField::MaxDuration,
            TaskOptionField::ApplicationEngine,
            TaskOptionField::from(0),
            TaskOptionField::from(77),
        ] {
            // Ours -> generated.
            let mut buf = Vec::new();
            ProtoField::encode_field(1, &value, &mut buf);
            let decoded = Ref::decode(buf.as_slice()).unwrap();
            assert_eq!(decoded.field.unwrap().field, i32::from(value));

            // Generated -> ours.
            let bytes = Ref {
                field: Some(crate::api::v3::sessions::TaskOptionField {
                    field: i32::from(value),
                }),
            }
            .encode_to_vec();
            let mut cursor = bytes.as_slice();
            let (tag, wire_type) = decode_key(&mut cursor).unwrap();
            assert_eq!(tag, 1);
            let mut back = TaskOptionField::default();
            ProtoField::merge_field(wire_type, &mut back, &mut cursor, DecodeContext::default())
                .unwrap();
            assert_eq!(back, value);
        }
    }
}
