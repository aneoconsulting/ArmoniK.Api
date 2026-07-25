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

impl From<TaskOptions> for v3::TaskOptions {
    fn from(value: TaskOptions) -> Self {
        Self {
            options: value.options,
            max_duration: Some(value.max_duration),
            max_retries: value.max_retries,
            priority: value.priority,
            partition_id: value.partition_id,
            application_name: value.application_name,
            application_version: value.application_version,
            application_namespace: value.application_namespace,
            application_service: value.application_service,
            engine_type: value.engine_type,
        }
    }
}

impl From<v3::TaskOptions> for TaskOptions {
    fn from(value: v3::TaskOptions) -> Self {
        Self {
            options: value.options,
            max_duration: value.max_duration.unwrap_or(INFINITE_DURATION),
            max_retries: value.max_retries,
            priority: value.priority,
            partition_id: value.partition_id,
            application_name: value.application_name,
            application_version: value.application_version,
            application_namespace: value.application_namespace,
            application_service: value.application_service,
            engine_type: value.engine_type,
        }
    }
}

super::impl_convert!(req TaskOptions : v3::TaskOptions);

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::{TaskOptions, INFINITE_DURATION};
    use crate::api::v3;

    /// The derived implementation must round-trip against the generated type.
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
        let theirs = v3::TaskOptions::decode(ours.encode_to_vec().as_slice()).unwrap();
        assert_eq!(theirs, v3::TaskOptions::from(ours.clone()));

        let back = TaskOptions::decode(theirs.encode_to_vec().as_slice()).unwrap();
        assert_eq!(back, ours);
        assert_eq!(back.options, ours.options);
    }

    /// `Message::decode` seeds with `Default::default()`, so a message with
    /// an absent `max_duration` decodes to `INFINITE_DURATION`, exactly like
    /// the historical `unwrap_or(INFINITE_DURATION)` conversion. And since
    /// `INFINITE_DURATION` does not encode to zero bytes, it is emitted on
    /// encode, matching the historical `Some(max_duration)`.
    #[test]
    fn custom_default_survives_via_merge_seeding() {
        let absent = v3::TaskOptions {
            max_duration: None,
            max_retries: 3,
            ..Default::default()
        };
        let ours = TaskOptions::decode(absent.encode_to_vec().as_slice()).unwrap();
        assert_eq!(ours.max_duration.seconds, INFINITE_DURATION.seconds);
        assert_eq!(ours.max_duration.nanos, INFINITE_DURATION.nanos);

        let reencoded = v3::TaskOptions::decode(ours.encode_to_vec().as_slice()).unwrap();
        assert_eq!(reencoded.max_duration, Some(INFINITE_DURATION));
    }

    /// A wire occurrence of `max_duration` must merge from the proto zero
    /// value, not from the `INFINITE_DURATION` seed: a partial duration
    /// (only nanos set) is a partial duration, exactly like the historical
    /// `unwrap_or` conversion produced.
    #[test]
    fn present_duration_does_not_inherit_the_seed() {
        let partial = v3::TaskOptions {
            max_duration: Some(prost_types::Duration {
                seconds: 0,
                nanos: 7,
            }),
            ..Default::default()
        };
        let ours = TaskOptions::decode(partial.encode_to_vec().as_slice()).unwrap();
        assert_eq!(ours.max_duration.seconds, 0);
        assert_eq!(ours.max_duration.nanos, 7);

        let explicit_zero = v3::TaskOptions {
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
            field: Option<v3::sessions::TaskOptionField>,
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
                field: Some(v3::sessions::TaskOptionField {
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
