from abc import ABC, abstractmethod
from typing import Any, cast

from google.protobuf.message import Message

from .filter import FilterWrapper, StringFilter, DurationFilter, NumberFilter, FType, _na

from ...protogen.common.tasks_fields_pb2 import (
    TaskField as rawTaskField,
    TaskSummaryField,
    TaskOptionGenericField,
    TaskOptionField,
    TASK_SUMMARY_ENUM_FIELD_TASK_ID,
    TASK_SUMMARY_ENUM_FIELD_SESSION_ID,
    TASK_SUMMARY_ENUM_FIELD_OWNER_POD_ID,
    TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID,
    TASK_SUMMARY_ENUM_FIELD_CREATED_BY,
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
    SessionRawField,
    SESSION_RAW_ENUM_FIELD_SESSION_ID,
    SESSION_RAW_ENUM_FIELD_DURATION,
    SESSION_RAW_ENUM_FIELD_CREATED_AT,
    SESSION_RAW_ENUM_FIELD_DELETED_AT,
    SESSION_RAW_ENUM_FIELD_PURGED_AT,
    SESSION_RAW_ENUM_FIELD_STATUS,
    SESSION_RAW_ENUM_FIELD_PARTITION_IDS,
    SESSION_RAW_ENUM_FIELD_CLIENT_SUBMISSION,
    SESSION_RAW_ENUM_FIELD_WORKER_SUBMISSION,
    SESSION_RAW_ENUM_FIELD_CLOSED_AT,
    SESSION_RAW_ENUM_FIELD_CANCELLED_AT,
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

from ...protogen.common.results_fields_pb2 import (
    ResultField,
    ResultRawField,
    RESULT_RAW_ENUM_FIELD_RESULT_ID,
    RESULT_RAW_ENUM_FIELD_SIZE,
    RESULT_RAW_ENUM_FIELD_NAME,
    RESULT_RAW_ENUM_FIELD_STATUS,
    RESULT_RAW_ENUM_FIELD_COMPLETED_AT,
    RESULT_RAW_ENUM_FIELD_CREATED_AT,
    RESULT_RAW_ENUM_FIELD_SESSION_ID,
    RESULT_RAW_ENUM_FIELD_CREATED_BY,
    RESULT_RAW_ENUM_FIELD_OWNER_TASK_ID,
)

from ...protogen.common.partitions_fields_pb2 import (
    PartitionField,
    PartitionRawField,
    PARTITION_RAW_ENUM_FIELD_ID,
    PARTITION_RAW_ENUM_FIELD_PRIORITY,
    PARTITION_RAW_ENUM_FIELD_PREEMPTION_PERCENTAGE,
    PARTITION_RAW_ENUM_FIELD_POD_RESERVED,
    PARTITION_RAW_ENUM_FIELD_PARENT_PARTITION_IDS,
    PARTITION_RAW_ENUM_FIELD_POD_MAX,
)

from ...protogen.common.applications_fields_pb2 import (
    ApplicationField,
    ApplicationRawField,
    APPLICATION_RAW_ENUM_FIELD_NAMESPACE,
    APPLICATION_RAW_ENUM_FIELD_SERVICE,
    APPLICATION_RAW_ENUM_FIELD_VERSION,
    APPLICATION_RAW_ENUM_FIELD_NAME,
)

from ...protogen.common.tasks_filters_pb2 import (
    FilterField as rawTaskFilterField,
    FilterStatus as rawTaskFilterStatus,
    Filters as rawTaskFilters,
    FiltersAnd as rawTaskFilterAnd,
)

from ...protogen.common.sessions_filters_pb2 import (
    FilterField as rawSessionFilterField,
    FilterStatus as rawSessionFilterStatus,
    Filters as rawSessionFilters,
    FiltersAnd as rawSessionFilterAnd,
)

from ...protogen.common.results_filters_pb2 import (
    FilterField as rawResultFilterField,
    FilterStatus as rawResultFilterStatus,
    Filters as rawResultFilters,
    FiltersAnd as rawResultFilterAnd,
)

from ...protogen.common.partitions_filters_pb2 import (
    FilterField as rawPartitionFilterField,
    Filters as rawPartitionFilters,
    FiltersAnd as rawPartitionFilterAnd,
)

from ...protogen.common.applications_filters_pb2 import (
    FilterField as rawApplicationFilterField,
    Filters as rawApplicationFilters,
    FiltersAnd as rawApplicationFilterAnd,
)

"""
This file defines the fields available for each object type. Used internally.
"""


class GenericTaskOptionsFilter(FilterWrapper, ABC):
    """
    Filter for task options
    """

    @abstractmethod
    def __getitem__(self, item: str) -> StringFilter: ...

    @abstractmethod
    def _build_field(self, field: Any) -> Message: ...

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
        return self._string(self._build_field(TASK_SUMMARY_ENUM_FIELD_ERROR))

    def _build_field(self, field: Any) -> Message:
        if field != TASK_SUMMARY_ENUM_FIELD_ERROR:
            _na("")
        return rawTaskField(task_summary_field=TaskSummaryField(field=field))


class TaskFilter(FilterWrapper):
    _fields = {
        "id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_TASK_ID),
        "session_id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_SESSION_ID),
        "owner_pod_id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_OWNER_POD_ID),
        "initial_task_id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID),
        "created_by": (FType.STR, TASK_SUMMARY_ENUM_FIELD_CREATED_BY),
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
        "creation_to_end_duration": (
            FType.DURATION,
            TASK_SUMMARY_ENUM_FIELD_CREATION_TO_END_DURATION,
        ),
        "processing_to_end_duration": (
            FType.DURATION,
            TASK_SUMMARY_ENUM_FIELD_PROCESSING_TO_END_DURATION,
        ),
        "received_to_end_duration": (
            FType.DURATION,
            TASK_SUMMARY_ENUM_FIELD_RECEIVED_TO_END_DURATION,
        ),
        "output": (FType.NA, "output"),
        "pod_hostname": (FType.STR, TASK_SUMMARY_ENUM_FIELD_POD_HOSTNAME),
        "payload_id": (FType.STR, TASK_SUMMARY_ENUM_FIELD_PAYLOAD_ID),
    }

    def __init__(self):
        super().__init__(rawTaskFilters, rawTaskFilterAnd, rawTaskFilterField, rawTaskFilterStatus)

    def _build_field(self, field: Any) -> Message:
        return rawTaskField(task_summary_field=TaskSummaryField(field=field))


class TaskOptionFilter(GenericTaskOptionsFilter):
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

    def _build_field(self, field: Any) -> Message:
        return rawTaskField(task_option_field=TaskOptionField(field=field))


class SessionFilter(FilterWrapper):
    _fields = {
        "session_id": (FType.STR, SESSION_RAW_ENUM_FIELD_SESSION_ID),
        "status": (FType.STATUS, SESSION_RAW_ENUM_FIELD_STATUS),
        "client_submission": (FType.BOOL, SESSION_RAW_ENUM_FIELD_CLIENT_SUBMISSION),
        "worker_submission": (FType.BOOL, SESSION_RAW_ENUM_FIELD_WORKER_SUBMISSION),
        "partition_ids": (FType.ARRAY, SESSION_RAW_ENUM_FIELD_PARTITION_IDS),
        "options": (FType.NA, "options"),
        "created_at": (FType.DATE, SESSION_RAW_ENUM_FIELD_CREATED_AT),
        "cancelled_at": (FType.DATE, SESSION_RAW_ENUM_FIELD_CANCELLED_AT),
        "closed_at": (FType.DATE, SESSION_RAW_ENUM_FIELD_CLOSED_AT),
        "purged_at": (FType.DATE, SESSION_RAW_ENUM_FIELD_PURGED_AT),
        "deleted_at": (FType.DATE, SESSION_RAW_ENUM_FIELD_DELETED_AT),
        "duration": (FType.DURATION, SESSION_RAW_ENUM_FIELD_DURATION),
    }

    def __init__(self):
        super().__init__(
            rawSessionFilters, rawSessionFilterAnd, rawSessionFilterField, rawSessionFilterStatus
        )

    def _build_field(self, field: Any) -> Message:
        return SessionField(session_raw_field=SessionRawField(field=field))


class SessionTaskOptionFilter(GenericTaskOptionsFilter):
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
        super().__init__(
            rawSessionFilters, rawSessionFilterAnd, rawSessionFilterField, rawSessionFilterStatus
        )

    def __getitem__(self, item: str) -> StringFilter:
        return self._string(
            SessionField(task_option_generic_field=SessionOptionGenericField(field=item))
        )

    def _build_field(self, field: Any) -> Message:
        return SessionField(task_option_field=SessionOptionField(field=field))


class ResultFilter(FilterWrapper):
    _fields = {
        "session_id": (FType.STR, RESULT_RAW_ENUM_FIELD_SESSION_ID),
        "status": (FType.STATUS, RESULT_RAW_ENUM_FIELD_STATUS),
        "name": (FType.STR, RESULT_RAW_ENUM_FIELD_NAME),
        "created_at": (FType.DATE, RESULT_RAW_ENUM_FIELD_CREATED_AT),
        "completed_at": (FType.DATE, RESULT_RAW_ENUM_FIELD_COMPLETED_AT),
        "result_id": (FType.STR, RESULT_RAW_ENUM_FIELD_RESULT_ID),
        "size": (FType.NUM, RESULT_RAW_ENUM_FIELD_SIZE),
        "created_by": (FType.STR, RESULT_RAW_ENUM_FIELD_CREATED_BY),
        "owner_task_id": (FType.STR, RESULT_RAW_ENUM_FIELD_OWNER_TASK_ID),
    }

    def __init__(self):
        super().__init__(
            rawResultFilters, rawResultFilterAnd, rawResultFilterField, rawResultFilterStatus
        )

    def _build_field(self, field: Any) -> Message:
        return ResultField(result_raw_field=ResultRawField(field=field))


class PartitionFilter(FilterWrapper):
    _fields = {
        "id": (FType.STR, PARTITION_RAW_ENUM_FIELD_ID),
        "priority": (FType.NUM, PARTITION_RAW_ENUM_FIELD_PRIORITY),
        "preemption_percentage": (FType.NUM, PARTITION_RAW_ENUM_FIELD_PREEMPTION_PERCENTAGE),
        "pod_reserved": (FType.NUM, PARTITION_RAW_ENUM_FIELD_POD_RESERVED),
        "pod_max": (FType.NUM, PARTITION_RAW_ENUM_FIELD_POD_MAX),
        "parent_partition_ids": (FType.ARRAY, PARTITION_RAW_ENUM_FIELD_PARENT_PARTITION_IDS),
        "pod_configuration": (FType.NA, "pod_configuration"),
    }

    def __init__(self):
        super().__init__(rawPartitionFilters, rawPartitionFilterAnd, rawPartitionFilterField)

    def _build_field(self, field: Any) -> Message:
        return PartitionField(partition_raw_field=PartitionRawField(field=field))


class ApplicationFilter(FilterWrapper):
    _fields = {
        "name": (FType.STR, APPLICATION_RAW_ENUM_FIELD_NAME),
        "namespace": (FType.STR, APPLICATION_RAW_ENUM_FIELD_NAMESPACE),
        "service": (FType.STR, APPLICATION_RAW_ENUM_FIELD_SERVICE),
        "version": (FType.STR, APPLICATION_RAW_ENUM_FIELD_VERSION),
    }

    def __init__(self):
        super().__init__(rawApplicationFilters, rawApplicationFilterAnd, rawApplicationFilterField)

    def _build_field(self, field: Any) -> Message:
        return ApplicationField(application_field=ApplicationRawField(field=field))
