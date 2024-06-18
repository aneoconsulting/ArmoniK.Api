from typing import Any

from .filter import (
    FilterWrapper,
    StringFilter,
    DurationFilter,
    NumberFilter,
    StatusFilter,
    BooleanFilter,
    ArrayFilter,
    DateFilter,
)
from ...protogen.common.sessions_fields_pb2 import (
    TaskOptionField,
    SessionField,
    SessionRawField,
    TaskOptionGenericField,
    TASK_OPTION_ENUM_FIELD_MAX_DURATION,
    TASK_OPTION_ENUM_FIELD_MAX_RETRIES,
    TASK_OPTION_ENUM_FIELD_PRIORITY,
    TASK_OPTION_ENUM_FIELD_PARTITION_ID,
    TASK_OPTION_ENUM_FIELD_APPLICATION_NAME,
    TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION,
    TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE,
    TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE,
    TASK_OPTION_ENUM_FIELD_ENGINE_TYPE,
    SESSION_RAW_ENUM_FIELD_SESSION_ID,
    SESSION_RAW_ENUM_FIELD_CANCELLED_AT,
    SESSION_RAW_ENUM_FIELD_DURATION,
    SESSION_RAW_ENUM_FIELD_CLOSED_AT,
    SESSION_RAW_ENUM_FIELD_PARTITION_IDS,
    SESSION_RAW_ENUM_FIELD_CLIENT_SUBMISSION,
    SESSION_RAW_ENUM_FIELD_WORKER_SUBMISSION,
    SESSION_RAW_ENUM_FIELD_CREATED_AT,
    SESSION_RAW_ENUM_FIELD_DELETED_AT,
    SESSION_RAW_ENUM_FIELD_PURGED_AT,
    SESSION_RAW_ENUM_FIELD_STATUS,
)
from ...protogen.common.sessions_filters_pb2 import (
    Filters,
    FiltersAnd,
    FilterField,
    FilterStatus,
)


def _raw_field(field: Any) -> SessionField:
    return SessionField(session_raw_field=SessionRawField(field=field))


def _task_option_field(field: Any) -> SessionField:
    return SessionField(task_option_field=TaskOptionField(field=field))


class SessionTaskOptionFilter(FilterWrapper):
    """
    Filter for session task options
    """

    def __init__(self):
        super().__init__(Filters, FiltersAnd, FilterField, FilterStatus)

    def __getitem__(self, item: str) -> StringFilter:
        return self._string(
            SessionField(task_option_generic_field=TaskOptionGenericField(field=item))
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
        return self._string(_task_option_field(TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE))

    @property
    def application_version(self) -> StringFilter:
        return self._string(_task_option_field(TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION))

    @property
    def application_service(self) -> StringFilter:
        return self._string(_task_option_field(TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE))

    @property
    def engine_type(self) -> StringFilter:
        return self._string(_task_option_field(TASK_OPTION_ENUM_FIELD_ENGINE_TYPE))


class SessionFilter(FilterWrapper):
    """
    Filter for sessions
    """

    def __init__(self):
        super().__init__(Filters, FiltersAnd, FilterField, FilterStatus)

    @property
    def session_id(self) -> StringFilter:
        return self._string(_raw_field(SESSION_RAW_ENUM_FIELD_SESSION_ID))

    @property
    def status(self) -> StatusFilter:
        return self._status(_raw_field(SESSION_RAW_ENUM_FIELD_STATUS))

    @property
    def client_submission(self) -> BooleanFilter:
        return self._bool(_raw_field(SESSION_RAW_ENUM_FIELD_CLIENT_SUBMISSION))

    @property
    def worker_submission(self) -> BooleanFilter:
        return self._bool(_raw_field(SESSION_RAW_ENUM_FIELD_WORKER_SUBMISSION))

    @property
    def partition_ids(self) -> ArrayFilter:
        return self._array(_raw_field(SESSION_RAW_ENUM_FIELD_PARTITION_IDS))

    @property
    def options(self) -> SessionTaskOptionFilter:
        return SessionTaskOptionFilter()

    @property
    def created_at(self) -> DateFilter:
        return self._date(_raw_field(SESSION_RAW_ENUM_FIELD_CREATED_AT))

    @property
    def cancelled_at(self) -> DateFilter:
        return self._date(_raw_field(SESSION_RAW_ENUM_FIELD_CANCELLED_AT))

    @property
    def closed_at(self) -> DateFilter:
        return self._date(_raw_field(SESSION_RAW_ENUM_FIELD_CLOSED_AT))

    @property
    def purged_at(self) -> DateFilter:
        return self._date(_raw_field(SESSION_RAW_ENUM_FIELD_PURGED_AT))

    @property
    def deleted_at(self) -> DateFilter:
        return self._date(_raw_field(SESSION_RAW_ENUM_FIELD_DELETED_AT))

    @property
    def duration(self) -> DurationFilter:
        return self._duration(_raw_field(SESSION_RAW_ENUM_FIELD_DURATION))
