from abc import ABC, abstractmethod
from enum import Enum, auto
from typing import Optional, Any, Tuple, Dict, cast

from google.protobuf.message import Message

from ._message_types import DisjunctionType, ConjunctionType, BasicMessageType, InnerMessageType
from .filter import FilterWrapper, Filter, StringFilter, DurationFilter, NumberFilter

from ...protogen.common.tasks_fields_pb2 import (
    TaskField as rawTaskField,
    TaskSummaryField,
    TaskOptionGenericField,
    TaskOptionField,
    TASK_SUMMARY_ENUM_FIELD_TASK_ID,
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
    TASK_SUMMARY_ENUM_FIELD_PAYLOAD_ID,
    TASK_OPTION_ENUM_FIELD_MAX_DURATION,
    TASK_OPTION_ENUM_FIELD_MAX_RETRIES,
    TASK_OPTION_ENUM_FIELD_PRIORITY,
    TASK_OPTION_ENUM_FIELD_PARTITION_ID,
    TASK_OPTION_ENUM_FIELD_APPLICATION_NAME,
    TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE,
    TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION,
    TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE,
    TASK_OPTION_ENUM_FIELD_ENGINE_TYPE,
)

from ...protogen.common.sessions_fields_pb2 import (
    SessionField,
    TaskOptionField as SessionOptionField,
    TaskOptionGenericField as SessionOptionGenericField,
    TASK_OPTION_ENUM_FIELD_MAX_RETRIES as SESSION_OPTION_MAX_RETRIES,
    TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE as SESSION_OPTION_APPLICATION_NAMESPACE,
    TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION as SESSION_OPTION_APPLICATION_VERSION,
    TASK_OPTION_ENUM_FIELD_PRIORITY as SESSION_OPTION_PRIORITY,
    TASK_OPTION_ENUM_FIELD_APPLICATION_NAME as SESSION_OPTION_APPLICATION_NAME,
    TASK_OPTION_ENUM_FIELD_PARTITION_ID as SESSION_OPTION_PARTITION_ID,
    TASK_OPTION_ENUM_FIELD_ENGINE_TYPE as SESSION_OPTION_ENGINE_TYPE,
    TASK_OPTION_ENUM_FIELD_MAX_DURATION as SESSION_OPTION_MAX_DURATION,
    TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE as SESSION_OPTION_APPLICATION_SERVICE,
)

from ...protogen.common.tasks_filters_pb2 import (
    FilterField as rawTaskFilterField,
    FilterStatus as rawTaskFilterStatus,
    Filters as rawTaskFilters,
    FiltersAnd as rawTaskFilterAnd
)

from ...protogen.common.sessions_filters_pb2 import (
    FilterField as rawSessionFilterField,
    FilterStatus as rawSessionFilterStatus,
    Filters as rawSessionFilters,
    FiltersAnd as rawSessionFilterAnd
)


class FType(Enum):
    UNKNOWN = auto()
    NA = auto()
    NUM = auto()
    STR = auto()
    ARRAY = auto()
    DURATION = auto()
    DATE = auto()
    STATUS = auto()
    BOOL = auto()


def _raise(field):
    msg = f"Unknown field {field}"
    raise ValueError(msg)


def _na(field):
    msg = f"Field {field} is not available as a filter"
    raise ValueError(msg)


class FilterConstructor(FilterWrapper, ABC):
    _fields: Dict[str, Tuple[FType, Optional[Any]]] = {}

    def __init__(
        self,
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: BasicMessageType,
        status_type: Optional[InnerMessageType] = None
    ):
        super().__init__(
            disjunction_message_type,
            conjunction_message_type,
            message_type,
            status_type=status_type,
        )
        self._vtable = {
            FType.UNKNOWN: _raise,
            FType.NA: _na,
            FType.NUM: self._number,
            FType.STR: self._string,
            FType.ARRAY: self._array,
            FType.DURATION: self._duration,
            FType.DATE: self._date,
            FType.STATUS: self._status,
            FType.BOOL: self._bool,
        }

    @abstractmethod
    def build_field(self, field: Any) -> Message: ...

    def __call__(self, field_name: str) -> Filter:
        ftype, field_value = self.__class__._fields.get(field_name, (FType.UNKNOWN, field_name))
        return self._vtable[ftype](self.build_field(field_value))


class _TaskOptionFilter(FilterConstructor, ABC):
    """
    Filter for task options
    """

    @abstractmethod
    def __getitem__(self, item: str) -> StringFilter: ...

    @abstractmethod
    def build_field(self, field: Any) -> Message: ...

    @property
    def max_duration(self) -> DurationFilter:
        return cast(DurationFilter, self("max_duration"))

    @property
    def max_retries(self) -> NumberFilter:
        return cast(NumberFilter, self("max_retries"))

    @property
    def priority(self) -> NumberFilter:
        return cast(NumberFilter, self("priority"))

    @property
    def partition_id(self) -> StringFilter:
        return cast(StringFilter, self("partition_id"))

    @property
    def application_name(self) -> StringFilter:
        return cast(StringFilter, self("application_name"))

    @property
    def application_namespace(self) -> StringFilter:
        return cast(StringFilter, self("application_namespace"))

    @property
    def application_version(self) -> StringFilter:
        return cast(StringFilter, self("application_version"))

    @property
    def application_service(self) -> StringFilter:
        return cast(StringFilter, self("application_service"))

    @property
    def engine_type(self) -> StringFilter:
        return cast(StringFilter, self("engine_type"))


class OutputFilter(FilterWrapper):
    def __init__(self):
        super().__init__(rawTaskFilters, rawTaskFilterAnd, rawTaskFilterField, rawTaskFilterStatus)

    @property
    def error(self) -> StringFilter:
        return self._string(rawTaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_ERROR)))


class TaskFilter(FilterConstructor):
    _fields = {
        "id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_TASK_ID),
        "session_id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_SESSION_ID),
        "owner_pod_id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_OWNER_POD_ID),
        "initial_task_id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID),
        "parent_task_ids": (FType.NA, "parent_task_ids"),
        "data_dependencies": (FType.NA, "data_dependencies"),
        "expected_output_ids": (FType.NA, "expected_output_ids"),
        "retry_of_ids": (FType.NA, "retry_of_ids"),

        "status": (FType.STATUS, TASK_SUMMARY_ENUM_FIELD_STATUS),
        "status_message": (FType.NA, "status_message"),

        "options": (FType.NA, "options"),
        "created_at": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_CREATED_AT),
        "submitted_at": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_SUBMITTED_AT),
        "received_at": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_RECEIVED_AT),
        "acquired_at": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_ACQUIRED_AT),
        "fetched_at": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_FETCHED_AT),
        "started_at": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_STARTED_AT),
        "processed_at": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_PROCESSED_AT),
        "ended_at": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_ENDED_AT),
        "pod_ttl": (FType.DATE, TASK_SUMMARY_ENUM_FIELD_POD_TTL),

        "creation_to_end_duration": (FType.DURATION, TASK_SUMMARY_ENUM_FIELD_CREATION_TO_END_DURATION),
        "processing_to_end_duration": (FType.DURATION, TASK_SUMMARY_ENUM_FIELD_PROCESSING_TO_END_DURATION),
        "received_to_end_duration": (FType.DURATION, TASK_SUMMARY_ENUM_FIELD_RECEIVED_TO_END_DURATION),

        "output": (FType.NA, "output"),

        "pod_hostname": (FType.STR, TASK_SUMMARY_ENUM_FIELD_POD_HOSTNAME),
        "payload_id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_PAYLOAD_ID)
    }

    def __init__(self):
        super().__init__(rawTaskFilters, rawTaskFilterAnd, rawTaskFilterField, rawTaskFilterStatus)

    def build_field(self, field: Any) -> Message:
        return rawTaskField(task_summary_field=TaskSummaryField(field=field))


class TaskOptionFilter(_TaskOptionFilter):
    _fields = {
        "max_duration": (FType.DURATION, TASK_OPTION_ENUM_FIELD_MAX_DURATION),
        "max_retries": (FType.NUM, TASK_OPTION_ENUM_FIELD_MAX_RETRIES),
        "priority": (FType.NUM, TASK_OPTION_ENUM_FIELD_PRIORITY),
        "partition_id": (FType.STR, TASK_OPTION_ENUM_FIELD_PARTITION_ID),
        "application_name": (FType.STR, TASK_OPTION_ENUM_FIELD_APPLICATION_NAME),
        "application_namespace": (FType.STR, TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE),
        "application_version": (FType.STR, TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION),
        "application_service": (FType.STR, TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE),
        "engine_type": (FType.STR, TASK_OPTION_ENUM_FIELD_ENGINE_TYPE),
    }

    def __init__(self):
        super().__init__(rawTaskFilters, rawTaskFilterAnd, rawTaskFilterField, rawTaskFilterStatus)

    def __getitem__(self, item: str) -> StringFilter:
        return self._string(
            rawTaskField(task_option_generic_field=TaskOptionGenericField(field=item))
        )

    def build_field(self, field: Any) -> Message:
        return rawTaskField(task_option_field=TaskOptionField(field=field))


class SessionFilter(FilterConstructor)


class SessionTaskOptionFilter(_TaskOptionFilter):
    _fields = {
        "max_duration": (FType.DURATION, SESSION_OPTION_MAX_DURATION),
        "max_retries": (FType.NUM, SESSION_OPTION_MAX_RETRIES),
        "priority": (FType.NUM, SESSION_OPTION_PRIORITY),
        "partition_id": (FType.STR, SESSION_OPTION_PARTITION_ID),
        "application_name": (FType.STR, SESSION_OPTION_APPLICATION_NAME),
        "application_namespace": (FType.STR, SESSION_OPTION_APPLICATION_NAMESPACE),
        "application_version": (FType.STR, SESSION_OPTION_APPLICATION_VERSION),
        "application_service": (FType.STR, SESSION_OPTION_APPLICATION_SERVICE),
        "engine_type": (FType.STR, SESSION_OPTION_ENGINE_TYPE),
    }

    def __init__(self):
        super().__init__(rawSessionFilters, rawSessionFilterAnd, rawSessionFilterField, rawSessionFilterStatus)

    def __getitem__(self, item: str) -> StringFilter:
        return self._string(
            SessionField(task_option_generic_field=SessionOptionGenericField(field=item))
        )

    def build_field(self, field: Any) -> Message:
        return SessionField(task_option_field=SessionOptionField(field=field))


