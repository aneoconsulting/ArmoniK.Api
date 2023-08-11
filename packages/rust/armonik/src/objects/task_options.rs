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
