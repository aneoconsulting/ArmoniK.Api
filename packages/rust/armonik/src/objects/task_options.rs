use ::std::collections::HashMap;

use crate::api::v3;

const INFINITE_DURATION: prost_types::Duration = prost_types::Duration {
    seconds: 315576000000,
    nanos: 0,
};

#[derive(Debug, Clone)]
pub struct TaskOptions {
    pub options: HashMap<String, String>,
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

super::impl_convert!(TaskOptions : Option<v3::TaskOptions>);

/// Represents a field in a task option.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum TaskOptionField {
    /// Unspecified.
    #[default]
    Unspecified = 0,
    MaxDuration = 1,
    MaxRetries = 2,
    Priority = 3,
    PartitionId = 4,
    ApplicationName = 5,
    ApplicationVersion = 6,
    ApplicationNamespace = 7,
    ApplicationService = 8,
    ApplicationEngine = 9,
}

impl From<i32> for TaskOptionField {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::MaxDuration,
            2 => Self::MaxRetries,
            3 => Self::Priority,
            4 => Self::PartitionId,
            5 => Self::ApplicationName,
            6 => Self::ApplicationVersion,
            7 => Self::ApplicationNamespace,
            8 => Self::ApplicationService,
            9 => Self::ApplicationEngine,
            _ => Self::Unspecified,
        }
    }
}

impl From<TaskOptionField> for v3::sessions::TaskOptionField {
    fn from(value: TaskOptionField) -> Self {
        Self {
            field: value as i32,
        }
    }
}

impl From<TaskOptionField> for v3::tasks::TaskOptionField {
    fn from(value: TaskOptionField) -> Self {
        Self {
            field: value as i32,
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

super::super::impl_convert!(TaskOptionField : Option<v3::sessions::TaskOptionField>);
super::super::impl_convert!(TaskOptionField : Option<v3::tasks::TaskOptionField>);
