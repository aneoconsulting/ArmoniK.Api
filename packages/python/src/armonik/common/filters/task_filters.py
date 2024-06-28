from typing import Any

from protogen.common.tasks_fields_pb2 import (
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
from protogen.common.tasks_filters_pb2 import (
    FilterField as rawFilterField,
)
from protogen.common.tasks_filters_pb2 import (
    Filters as rawFilters,
)
from protogen.common.tasks_filters_pb2 import (
    FiltersAnd as rawFilterAnd,
)
from protogen.common.tasks_filters_pb2 import (
    FilterStatus as rawFilterStatus,
)
from .filter import (
    StringFilter,
    FilterWrapper,
    StatusFilter,
    DateFilter,
    DurationFilter,
    NumberFilter,
)


def _summary_field(field: Any) -> TaskField:
    return TaskField(task_summary_field=TaskSummaryField(field=field))


def _task_option_field(field: Any) -> TaskField:
    return TaskField(task_option_field=TaskOptionField(field=field))


class TaskOptionFilter(FilterWrapper):
    def __init__(self):
        super().__init__(rawFilters, rawFilterAnd, rawFilterField, rawFilterStatus)

    def __getitem__(self, item: str) -> StringFilter:
        return self._string(
            TaskField(task_option_generic_field=TaskOptionGenericField(field=item))
        )

    @property
    def max_duration(self) -> DurationFilter:
        return self._duration(_task_option_field(TASK_OPTION_ENUM_FIELD_MAX_DURATION))

    @property
    def max_retries(self) -> NumberFilter:
        return self._number(_task_option_field(TASK_OPTION_ENUM_FIELD_MAX_RETRIES))

    @property
    def priority(self) -> NumberFilter:
        return self._number(_task_option_field(TASK_OPTION_ENUM_FIELD_PRIORITY))

    @property
    def partition_id(self) -> StringFilter:
        return self._string(_task_option_field(TASK_OPTION_ENUM_FIELD_PARTITION_ID))

    @property
    def application_name(self) -> StringFilter:
        return self._string(_task_option_field(TASK_OPTION_ENUM_FIELD_APPLICATION_NAME))

    @property
    def application_namespace(self) -> StringFilter:
        return self._string(
            _task_option_field(TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE)
        )

    @property
    def application_version(self) -> StringFilter:
        return self._string(
            _task_option_field(TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION)
        )

    @property
    def application_service(self) -> StringFilter:
        return self._string(
            _task_option_field(TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE)
        )

    @property
    def engine_type(self) -> StringFilter:
        return self._string(_task_option_field(TASK_OPTION_ENUM_FIELD_ENGINE_TYPE))


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
        return TaskOptionFilter()

    @property
    def created_at(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_CREATED_AT))

    @property
    def submitted_at(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_SUBMITTED_AT))

    @property
    def received_at(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_RECEIVED_AT))

    @property
    def acquired_at(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_ACQUIRED_AT))

    @property
    def fetched_at(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_FETCHED_AT))

    @property
    def started_at(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_STARTED_AT))

    @property
    def processed_at(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_PROCESSED_AT))

    @property
    def ended_at(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_ENDED_AT))

    @property
    def pod_ttl(self) -> DateFilter:
        return self._date(_summary_field(TASK_SUMMARY_ENUM_FIELD_POD_TTL))

    @property
    def pod_hostname(self) -> StringFilter:
        return self._string(_summary_field(TASK_SUMMARY_ENUM_FIELD_POD_HOSTNAME))

    @property
    def creation_to_end_duration(self) -> DurationFilter:
        return self._duration(
            _summary_field(TASK_SUMMARY_ENUM_FIELD_CREATION_TO_END_DURATION)
        )

    @property
    def processing_to_end_duration(self) -> DurationFilter:
        return self._duration(
            _summary_field(TASK_SUMMARY_ENUM_FIELD_PROCESSING_TO_END_DURATION)
        )

    @property
    def received_to_end_duration(self) -> DurationFilter:
        return self._duration(
            _summary_field(TASK_SUMMARY_ENUM_FIELD_RECEIVED_TO_END_DURATION)
        )

    @property
    def error(self) -> StringFilter:
        return self._string(_summary_field(TASK_SUMMARY_ENUM_FIELD_ERROR))
