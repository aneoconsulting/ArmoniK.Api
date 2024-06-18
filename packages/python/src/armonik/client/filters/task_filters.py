from typing import Any

from ...protogen.common.tasks_fields_pb2 import (
    TaskField,
    TaskSummaryField,
    TASK_SUMMARY_ENUM_FIELD_TASK_ID,
    TaskOptionField,
    TaskOptionGenericField,
    TASK_SUMMARY_ENUM_FIELD_SESSION_ID,
    TASK_SUMMARY_ENUM_FIELD_OWNER_POD_ID,
    TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID,
    TASK_SUMMARY_ENUM_FIELD_STATUS,
    TASK_SUMMARY_ENUM_FIELD_CREATED_AT,
    TASK_SUMMARY_ENUM_FIELD_SUBMITTED_AT,
    TASK_SUMMARY_ENUM_FIELD_STARTED_AT,
    TASK_SUMMARY_ENUM_FIELD_ENDED_AT,
    TASK_SUMMARY_ENUM_FIELD_CREATION_TO_END_DURATION,
    TASK_SUMMARY_ENUM_FIELD_PROCESSING_TO_END_DURATION,
    TASK_SUMMARY_ENUM_FIELD_RECEIVED_TO_END_DURATION,
    TASK_SUMMARY_ENUM_FIELD_POD_TTL,
    TASK_SUMMARY_ENUM_FIELD_POD_HOSTNAME,
    TASK_SUMMARY_ENUM_FIELD_RECEIVED_AT,
    TASK_SUMMARY_ENUM_FIELD_ACQUIRED_AT,
    TASK_SUMMARY_ENUM_FIELD_PROCESSED_AT,
    TASK_SUMMARY_ENUM_FIELD_ERROR,
    TASK_SUMMARY_ENUM_FIELD_FETCHED_AT,
    TASK_OPTION_ENUM_FIELD_UNSPECIFIED,
    TASK_OPTION_ENUM_FIELD_MAX_DURATION,
    TASK_OPTION_ENUM_FIELD_MAX_RETRIES,
    TASK_OPTION_ENUM_FIELD_PRIORITY,
    TASK_OPTION_ENUM_FIELD_PARTITION_ID,
    TASK_OPTION_ENUM_FIELD_APPLICATION_NAME,
    TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION,
    TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE,
    TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE,
    TASK_OPTION_ENUM_FIELD_ENGINE_TYPE,
)
from ...protogen.common.tasks_filters_pb2 import (
    FilterField as rawFilterField,
)
from ...protogen.common.tasks_filters_pb2 import (
    Filters as rawFilters,
)
from ...protogen.common.tasks_filters_pb2 import (
    FiltersAnd as rawFilterAnd,
)
from ...protogen.common.tasks_filters_pb2 import (
    FilterStatus as rawFilterStatus,
)
from .filter import StringFilter, FilterWrapper, ArrayFilter, StatusFilter


def _summary_field(field: Any) -> TaskField:
    return TaskField(task_summary_field=TaskSummaryField(field=field))


def _task_option_field(field: Any) -> TaskField:
    return TaskField(task_option_field=TaskOptionField(field=field))


class TaskOptionFilter:
    def __init__(self, wrapper: FilterWrapper):
        self.wrapper = wrapper

    def __getitem__(self, item: str) -> StringFilter:
        return self.wrapper._string(
            TaskField(task_option_generic_field=TaskOptionGenericField(field=item))
        )


class TaskFilter(FilterWrapper):

    def __init__(self):
        super().__init__(rawFilters, rawFilterAnd, rawFilterField, rawFilterStatus)

    @property
    def id(self) -> StringFilter:
        return self._string(_summary_field(TASK_SUMMARY_ENUM_FIELD_TASK_ID))

    @property
    def session_id(self) -> StringFilter:
        return self._string(_summary_field(TASK_SUMMARY_ENUM_FIELD_SESSION_ID))

    @property
    def owner_pod_id(self) -> StringFilter:
        return self._string(_summary_field(TASK_SUMMARY_ENUM_FIELD_OWNER_POD_ID))

    @property
    def initial_task_id(self) -> StringFilter:
        return self._string(_summary_field(TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID))

    @property
    def status(self) -> StatusFilter:
        return self._status(_summary_field(TASK_SUMMARY_ENUM_FIELD_STATUS))

    @property
    def options(self) -> TaskOptionFilter:
        return TaskOptionFilter(self)

    id: Optional[str] = None
    session_id: Optional[str] = None
    owner_pod_id: Optional[str] = None

    initial_task_id: Optional[str] = None
    parent_task_ids: List[str] = field(default_factory=list)
    data_dependencies: List[str] = field(default_factory=list)
    expected_output_ids: List[str] = field(default_factory=list)
    retry_of_ids: List[str] = field(default_factory=list)

    status: RawTaskStatus = TaskStatus.UNSPECIFIED
    status_message: Optional[str] = None

    options: Optional[TaskOptions] = None
    created_at: Optional[datetime] = None
    submitted_at: Optional[datetime] = None
    received_at: Optional[datetime] = None
    acquired_at: Optional[datetime] = None
    fetched_at: Optional[datetime] = None
    started_at: Optional[datetime] = None
    processed_at: Optional[datetime] = None
    ended_at: Optional[datetime] = None
    pod_ttl: Optional[datetime] = None

    creation_to_end_duration: Optional[timedelta] = timedelta(0)
    processing_to_end_duration: Optional[timedelta] = timedelta(0)
    received_to_end_duration: Optional[timedelta] = timedelta(0)

    output: Optional[Output] = None

    pod_hostname: Optional[str] = None
