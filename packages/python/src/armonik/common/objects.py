from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Dict, List, Optional

from deprecation import deprecated

from ..protogen.common.agent_common_pb2 import ResultMetaData
from ..protogen.common.applications_common_pb2 import ApplicationRaw
from ..protogen.common.tasks_common_pb2 import TaskDetailed, TaskSummary
from .filter import (
    FilterDescriptor,
    GenericTaskOptionsFilter,
    TaskFilter,
    TaskOptionFilter,
    SessionTaskOptionFilter,
    OutputFilter,
    SessionFilter,
    ResultFilter,
    PartitionFilter,
    ApplicationFilter,
)
from ..protogen.common.objects_pb2 import (
    Empty,
)
from ..protogen.common.objects_pb2 import (
    Output as WorkerOutput,
)
from ..protogen.common.objects_pb2 import (
    TaskOptions as RawTaskOptions,
)
from ..protogen.common.partitions_common_pb2 import PartitionRaw
from ..protogen.common.result_status_pb2 import ResultStatus as RawResultStatus
from ..protogen.common.results_common_pb2 import ResultRaw
from ..protogen.common.session_status_pb2 import SessionStatus as RawSessionStatus
from ..protogen.common.sessions_common_pb2 import SessionRaw
from .enumwrapper import ResultStatus, SessionStatus, TaskStatus
from .helpers import duration_to_timedelta, timedelta_to_duration, timestamp_to_datetime


@dataclass()
class TaskOptions:
    max_duration: timedelta
    priority: int
    max_retries: int
    partition_id: Optional[str] = None
    application_name: Optional[str] = None
    application_version: Optional[str] = None
    application_namespace: Optional[str] = None
    application_service: Optional[str] = None
    engine_type: Optional[str] = None
    options: Dict[str, str] = field(default_factory=dict)

    @classmethod
    def from_message(cls, task_options):
        return cls(
            max_duration=duration_to_timedelta(task_options.max_duration),
            max_retries=task_options.max_retries,
            priority=task_options.priority,
            partition_id=task_options.partition_id,
            application_name=task_options.application_name,
            application_version=task_options.application_version,
            application_namespace=task_options.application_namespace,
            application_service=task_options.application_service,
            engine_type=task_options.engine_type,
            options={k: v for k, v in task_options.options.items()},
        )

    def to_message(self) -> RawTaskOptions:
        return RawTaskOptions(
            max_duration=timedelta_to_duration(self.max_duration),
            max_retries=self.max_retries,
            priority=self.priority,
            partition_id=self.partition_id,
            application_name=self.application_name,
            application_version=self.application_version,
            application_namespace=self.application_namespace,
            application_service=self.application_service,
            engine_type=self.engine_type,
            options=self.options,
        )


@dataclass()
class Output:
    error: Optional[str] = None

    @property
    def success(self) -> bool:
        return self.error is None

    def to_message(self):
        if self.error is None:
            return WorkerOutput(ok=Empty())
        return WorkerOutput(error=WorkerOutput.Error(details=self.error))


@dataclass()
class TaskDefinition:
    payload_id: str = ""
    expected_output_ids: List[str] = field(default_factory=list)
    data_dependencies: List[str] = field(default_factory=list)
    options: Optional[TaskOptions] = None

    def __post_init__(self):
        if len(self.expected_output_ids) <= 0:
            raise ValueError("expected_output_ids must be not be empty")


_task_filter = TaskFilter()


class TaskOptionDescriptor(FilterDescriptor):
    def __get__(self, instance, owner) -> GenericTaskOptionsFilter:
        # When called statically, returns the Task option filter
        if instance is None:
            return TaskOptionFilter() if issubclass(owner, Task) else SessionTaskOptionFilter()
        # Otherwise gets the object's hidden property
        return getattr(instance, self._name)


class OutputDescriptor(FilterDescriptor):
    def __get__(self, instance, owner) -> OutputFilter:
        # When called statically, returns the Task option filter
        if instance is None:
            return OutputFilter()
        # Otherwise gets the object's hidden property
        return getattr(instance, self._name)


class Task:
    id = FilterDescriptor(_task_filter)
    session_id = FilterDescriptor(_task_filter)
    owner_pod_id = FilterDescriptor(_task_filter)

    initial_task_id = FilterDescriptor(_task_filter)
    created_by = FilterDescriptor(_task_filter)
    parent_task_ids = FilterDescriptor(_task_filter)
    data_dependencies = FilterDescriptor(_task_filter)
    expected_output_ids = FilterDescriptor(_task_filter)
    retry_of_ids = FilterDescriptor(_task_filter)

    status = FilterDescriptor(_task_filter)
    status_message = FilterDescriptor(_task_filter)

    options = TaskOptionDescriptor(_task_filter)
    created_at = FilterDescriptor(_task_filter)
    submitted_at = FilterDescriptor(_task_filter)
    received_at = FilterDescriptor(_task_filter)
    acquired_at = FilterDescriptor(_task_filter)
    fetched_at = FilterDescriptor(_task_filter)
    started_at = FilterDescriptor(_task_filter)
    processed_at = FilterDescriptor(_task_filter)
    ended_at = FilterDescriptor(_task_filter)
    pod_ttl = FilterDescriptor(_task_filter)

    creation_to_end_duration = FilterDescriptor(_task_filter)
    processing_to_end_duration = FilterDescriptor(_task_filter)
    received_to_end_duration = FilterDescriptor(_task_filter)

    output = OutputDescriptor(_task_filter)

    pod_hostname = FilterDescriptor(_task_filter)
    payload_id = FilterDescriptor(_task_filter)

    def __init__(
        self,
        id: Optional[str] = None,
        session_id: Optional[str] = None,
        owner_pod_id: Optional[str] = None,
        initial_task_id: Optional[str] = None,
        created_by: Optional[str] = None,
        parent_task_ids: Optional[List[str]] = None,
        data_dependencies: Optional[List[str]] = None,
        expected_output_ids: Optional[List[str]] = None,
        retry_of_ids: Optional[List[str]] = None,
        status: TaskStatus = TaskStatus.UNSPECIFIED,
        status_message: Optional[str] = None,
        options: Optional[TaskOptions] = None,
        created_at: Optional[datetime] = None,
        submitted_at: Optional[datetime] = None,
        received_at: Optional[datetime] = None,
        acquired_at: Optional[datetime] = None,
        fetched_at: Optional[datetime] = None,
        started_at: Optional[datetime] = None,
        processed_at: Optional[datetime] = None,
        ended_at: Optional[datetime] = None,
        pod_ttl: Optional[datetime] = None,
        creation_to_end_duration: timedelta = timedelta(0),
        processing_to_end_duration: timedelta = timedelta(0),
        received_to_end_duration: timedelta = timedelta(0),
        output: Optional[Output] = None,
        pod_hostname: Optional[str] = None,
        payload_id: Optional[str] = None,
    ):
        self.id = id
        self.session_id = session_id
        self.owner_pod_id = owner_pod_id
        self.initial_task_id = initial_task_id
        self.created_by = created_by
        self.parent_task_ids = parent_task_ids
        self.count_parent_task_ids = (
            len(self.parent_task_ids) if self.parent_task_ids is not None else None
        )
        self.data_dependencies = data_dependencies
        self.count_data_dependencies = (
            len(self.data_dependencies) if self.data_dependencies is not None else None
        )
        self.expected_output_ids = expected_output_ids
        self.count_expected_output_ids = (
            len(self.expected_output_ids) if self.expected_output_ids is not None else None
        )
        self.retry_of_ids = retry_of_ids
        self.count_retry_of_ids = len(self.retry_of_ids) if self.retry_of_ids is not None else None
        self.status = status
        self.status_message = status_message
        self.options = options
        self.created_at = created_at
        self.submitted_at = submitted_at
        self.received_at = received_at
        self.acquired_at = acquired_at
        self.fetched_at = fetched_at
        self.started_at = started_at
        self.processed_at = processed_at
        self.ended_at = ended_at
        self.pod_ttl = pod_ttl
        self.creation_to_end_duration = creation_to_end_duration
        self.processing_to_end_duration = processing_to_end_duration
        self.received_to_end_duration = received_to_end_duration
        self.output = output
        self.pod_hostname = pod_hostname
        self.payload_id = payload_id

    def refresh(self, task_client) -> None:
        """Refresh the fields of this task object by using the given task client

        Args:
            task_client: ArmoniKTasks client
        """
        result: "Task" = task_client.get_task(self.id)
        self.session_id = result.session_id
        self.owner_pod_id = result.owner_pod_id

        self.initial_task_id = result.initial_task_id
        self.created_by = result.created_by
        self.parent_task_ids = result.parent_task_ids
        self.data_dependencies = result.data_dependencies
        self.expected_output_ids = result.expected_output_ids
        self.retry_of_ids = result.retry_of_ids

        self.status = result.status
        self.status_message = result.status_message

        self.options = result.options
        self.created_at = result.created_at
        self.submitted_at = result.submitted_at
        self.received_at = result.received_at
        self.acquired_at = result.acquired_at
        self.fetched_at = result.fetched_at
        self.started_at = result.started_at
        self.processed_at = result.processed_at
        self.ended_at = result.ended_at
        self.pod_ttl = result.pod_ttl

        self.creation_to_end_duration = result.creation_to_end_duration
        self.processing_to_end_duration = result.processing_to_end_duration
        self.received_to_end_duration = result.received_to_end_duration

        self.output = result.output

        self.pod_hostname = result.pod_hostname
        self.payload_id = result.payload_id
        self.is_init = True

    @classmethod
    def from_message(cls, task_raw: TaskDetailed) -> "Task":
        return cls(
            id=task_raw.id,
            session_id=task_raw.session_id,
            owner_pod_id=task_raw.owner_pod_id,
            initial_task_id=task_raw.initial_task_id,
            created_by=task_raw.created_by,
            parent_task_ids=list(task_raw.parent_task_ids),
            data_dependencies=list(task_raw.data_dependencies),
            expected_output_ids=list(task_raw.expected_output_ids),
            retry_of_ids=list(task_raw.retry_of_ids),
            status=task_raw.status,
            status_message=task_raw.status_message,
            options=TaskOptions.from_message(task_raw.options),
            created_at=timestamp_to_datetime(task_raw.created_at),
            submitted_at=timestamp_to_datetime(task_raw.submitted_at),
            received_at=timestamp_to_datetime(task_raw.received_at),
            acquired_at=timestamp_to_datetime(task_raw.acquired_at),
            fetched_at=timestamp_to_datetime(task_raw.fetched_at),
            started_at=timestamp_to_datetime(task_raw.started_at),
            processed_at=timestamp_to_datetime(task_raw.processed_at),
            ended_at=timestamp_to_datetime(task_raw.ended_at),
            pod_ttl=timestamp_to_datetime(task_raw.pod_ttl),
            creation_to_end_duration=duration_to_timedelta(task_raw.creation_to_end_duration),
            processing_to_end_duration=duration_to_timedelta(task_raw.processing_to_end_duration),
            received_to_end_duration=duration_to_timedelta(task_raw.received_to_end_duration),
            output=Output(error=(task_raw.output.error if not task_raw.output.success else None)),
            pod_hostname=task_raw.pod_hostname,
            payload_id=task_raw.payload_id,
        )

    @staticmethod
    def from_summary(task_raw: TaskSummary) -> "Task":
        task = Task(
            id=task_raw.id,
            session_id=task_raw.session_id,
            owner_pod_id=task_raw.owner_pod_id,
            initial_task_id=task_raw.initial_task_id,
            created_by=task_raw.created_by,
            parent_task_ids=None,
            data_dependencies=None,
            expected_output_ids=None,
            retry_of_ids=None,
            status=task_raw.status,
            status_message=task_raw.status_message,
            options=TaskOptions.from_message(task_raw.options),
            created_at=timestamp_to_datetime(task_raw.created_at),
            submitted_at=timestamp_to_datetime(task_raw.submitted_at),
            received_at=timestamp_to_datetime(task_raw.received_at),
            acquired_at=timestamp_to_datetime(task_raw.acquired_at),
            fetched_at=timestamp_to_datetime(task_raw.fetched_at),
            started_at=timestamp_to_datetime(task_raw.started_at),
            processed_at=timestamp_to_datetime(task_raw.processed_at),
            ended_at=timestamp_to_datetime(task_raw.ended_at),
            pod_ttl=timestamp_to_datetime(task_raw.pod_ttl),
            creation_to_end_duration=duration_to_timedelta(task_raw.creation_to_end_duration),
            processing_to_end_duration=duration_to_timedelta(task_raw.processing_to_end_duration),
            received_to_end_duration=duration_to_timedelta(task_raw.received_to_end_duration),
            output=Output(
                error=(
                    task_raw.error
                    if task_raw.error is not None and len(task_raw.error) > 0
                    else None
                )
            ),
            pod_hostname=task_raw.pod_hostname,
            payload_id=task_raw.payload_id,
        )
        task.count_parent_task_ids = task_raw.count_parent_task_ids
        task.count_data_dependencies = task_raw.count_data_dependencies
        task.count_expected_output_ids = task_raw.count_expected_output_ids
        task.count_retry_of_ids = task_raw.count_retry_of_ids
        return task

    def __eq__(self, other: "Task") -> bool:
        return (
            self.id == other.id
            and self.session_id == other.session_id
            and self.owner_pod_id == other.owner_pod_id
            and self.initial_task_id == other.initial_task_id
            and self.created_by == other.created_by
            and self.parent_task_ids == other.parent_task_ids
            and self.data_dependencies == other.data_dependencies
            and self.expected_output_ids == other.expected_output_ids
            and self.retry_of_ids == other.retry_of_ids
            and self.status == other.status
            and self.status_message == other.status_message
            and self.options == other.options
            and self.created_at == other.created_at
            and self.submitted_at == other.submitted_at
            and self.received_at == other.received_at
            and self.acquired_at == other.acquired_at
            and self.fetched_at == other.fetched_at
            and self.started_at == other.started_at
            and self.processed_at == other.processed_at
            and self.ended_at == other.ended_at
            and self.pod_ttl == other.pod_ttl
            and self.creation_to_end_duration == other.creation_to_end_duration
            and self.processing_to_end_duration == other.processing_to_end_duration
            and self.received_to_end_duration == other.received_to_end_duration
            and self.output == other.output
            and self.pod_hostname == other.pod_hostname
            and self.payload_id == other.payload_id
        )


@deprecated(deprecated_in="3.14.0", details="Use sessions, task and results client instead")
@dataclass
class ResultAvailability:
    errors: List[str] = field(default_factory=list)

    def is_available(self) -> bool:
        return len(self.errors) == 0


_sessionFilter = SessionFilter()


class Session:
    session_id = FilterDescriptor(_sessionFilter)
    status = FilterDescriptor(_sessionFilter)
    client_submission = FilterDescriptor(_sessionFilter)
    worker_submission = FilterDescriptor(_sessionFilter)
    partition_ids = FilterDescriptor(_sessionFilter)
    options = TaskOptionDescriptor(_sessionFilter)
    created_at = FilterDescriptor(_sessionFilter)
    cancelled_at = FilterDescriptor(_sessionFilter)
    closed_at = FilterDescriptor(_sessionFilter)
    purged_at = FilterDescriptor(_sessionFilter)
    deleted_at = FilterDescriptor(_sessionFilter)
    duration = FilterDescriptor(_sessionFilter)

    def __init__(
        self,
        session_id: Optional[str] = None,
        status: RawSessionStatus = SessionStatus.UNSPECIFIED,
        client_submission: Optional[bool] = None,
        worker_submission: Optional[bool] = None,
        partition_ids: Optional[List[str]] = None,
        options: Optional[TaskOptions] = None,
        created_at: Optional[datetime] = None,
        cancelled_at: Optional[datetime] = None,
        closed_at: Optional[datetime] = None,
        purged_at: Optional[datetime] = None,
        deleted_at: Optional[datetime] = None,
        duration: Optional[timedelta] = None,
    ):
        self.session_id = session_id
        self.status = status
        self.client_submission = client_submission
        self.worker_submission = worker_submission
        self.partition_ids = partition_ids if partition_ids is not None else []
        self.options = options
        self.created_at = created_at
        self.cancelled_at = cancelled_at
        self.closed_at = closed_at
        self.purged_at = purged_at
        self.deleted_at = deleted_at
        self.duration = duration

    @classmethod
    def from_message(cls, session_raw: SessionRaw) -> "Session":
        return cls(
            session_id=session_raw.session_id,
            status=session_raw.status,
            client_submission=session_raw.client_submission,
            worker_submission=session_raw.worker_submission,
            partition_ids=list(session_raw.partition_ids),
            options=TaskOptions.from_message(session_raw.options),
            created_at=timestamp_to_datetime(session_raw.created_at),
            cancelled_at=timestamp_to_datetime(session_raw.cancelled_at),
            closed_at=timestamp_to_datetime(session_raw.closed_at),
            purged_at=timestamp_to_datetime(session_raw.purged_at),
            deleted_at=timestamp_to_datetime(session_raw.deleted_at),
            duration=duration_to_timedelta(session_raw.duration),
        )

    def __eq__(self, other: "Session") -> bool:
        return (
            self.session_id == other.session_id
            and self.status == other.status
            and self.client_submission == other.client_submission
            and self.worker_submission == other.worker_submission
            and self.partition_ids == other.partition_ids
            and self.options == other.options
            and self.created_at == other.created_at
            and self.cancelled_at == other.cancelled_at
            and self.closed_at == other.closed_at
            and self.purged_at == other.purged_at
            and self.deleted_at == other.deleted_at
            and self.duration == other.duration
        )


_resultFilter = ResultFilter()


class Result:
    session_id = FilterDescriptor(_resultFilter)
    name = FilterDescriptor(_resultFilter)
    created_by = FilterDescriptor(_resultFilter)
    owner_task_id = FilterDescriptor(_resultFilter)
    status = FilterDescriptor(_resultFilter)
    created_at = FilterDescriptor(_resultFilter)
    completed_at = FilterDescriptor(_resultFilter)
    result_id = FilterDescriptor(_resultFilter)
    size = FilterDescriptor(_resultFilter)

    def __init__(
        self,
        session_id: Optional[str] = None,
        name: Optional[str] = None,
        created_by: Optional[str] = None,
        owner_task_id: Optional[str] = None,
        status: RawResultStatus = ResultStatus.UNSPECIFIED,
        created_at: Optional[datetime] = None,
        completed_at: Optional[datetime] = None,
        result_id: Optional[str] = None,
        size: Optional[int] = None,
    ):
        self.session_id = session_id
        self.name = name
        self.created_by = created_by
        self.owner_task_id = owner_task_id
        self.status = status
        self.created_at = created_at
        self.completed_at = completed_at
        self.result_id = result_id
        self.size = size

    @classmethod
    def from_message(cls, result_raw: ResultRaw) -> "Result":
        return cls(
            session_id=result_raw.session_id,
            name=result_raw.name,
            created_by=result_raw.created_by,
            owner_task_id=result_raw.owner_task_id,
            status=result_raw.status,
            created_at=timestamp_to_datetime(result_raw.created_at),
            completed_at=timestamp_to_datetime(result_raw.completed_at),
            result_id=result_raw.result_id,
            size=result_raw.size,
        )

    @classmethod
    def from_result_metadata(cls, result_metadata: ResultMetaData) -> "Result":
        return cls(
            session_id=result_metadata.session_id,
            name=result_metadata.name,
            status=result_metadata.status,
            created_at=timestamp_to_datetime(result_metadata.created_at),
            result_id=result_metadata.result_id,
        )

    def __eq__(self, other: "Result") -> bool:
        return (
            self.session_id == other.session_id
            and self.status == other.status
            and self.result_id == other.result_id
            and self.created_at == other.created_at
            and self.completed_at == other.completed_at
            and self.result_id == other.result_id
            and self.size == other.size
        )


_partitionFilter = PartitionFilter()


class Partition:
    id = FilterDescriptor(_partitionFilter)
    parent_partition_ids = FilterDescriptor(_partitionFilter)
    pod_reserved = FilterDescriptor(_partitionFilter)
    pod_max = FilterDescriptor(_partitionFilter)
    pod_configuration = FilterDescriptor(_partitionFilter)
    preemption_percentage = FilterDescriptor(_partitionFilter)
    priority = FilterDescriptor(_partitionFilter)

    def __init__(
        self,
        id: str,
        parent_partition_ids: List[str],
        pod_reserved: int,
        pod_max: int,
        pod_configuration: Dict[str, str],
        preemption_percentage: int,
        priority: int,
    ):
        self.id = id
        self.parent_partition_ids = parent_partition_ids
        self.pod_reserved = pod_reserved
        self.pod_max = pod_max
        self.pod_configuration = pod_configuration
        self.preemption_percentage = preemption_percentage
        self.priority = priority

    @classmethod
    def from_message(cls, partition_raw: PartitionRaw) -> "Partition":
        return cls(
            id=partition_raw.id,
            parent_partition_ids=partition_raw.parent_partition_ids,
            pod_reserved=partition_raw.pod_reserved,
            pod_max=partition_raw.pod_max,
            pod_configuration=partition_raw.pod_configuration,
            preemption_percentage=partition_raw.preemption_percentage,
            priority=partition_raw.priority,
        )

    def __eq__(self, other: "Partition") -> bool:
        return (
            self.id == other.id
            and self.parent_partition_ids == other.parent_partition_ids
            and self.pod_reserved == other.pod_reserved
            and self.pod_max == other.pod_max
            and self.pod_configuration == other.pod_configuration
            and self.preemption_percentage == other.preemption_percentage
            and self.priority == other.priority
        )


_applicationFilter = ApplicationFilter()


class Application:
    name = FilterDescriptor(_applicationFilter)
    namespace = FilterDescriptor(_applicationFilter)
    service = FilterDescriptor(_applicationFilter)
    version = FilterDescriptor(_applicationFilter)

    def __init__(self, name: str, namespace: str, service: str, version: str):
        self.name = name
        self.namespace = namespace
        self.service = service
        self.version = version

    @classmethod
    def from_message(cls, application_raw: ApplicationRaw) -> "Application":
        return cls(
            name=application_raw.name,
            namespace=application_raw.namespace,
            service=application_raw.service,
            version=application_raw.version,
        )

    def __eq__(self, other: "Application") -> bool:
        return (
            self.name == other.name
            and self.namespace == other.namespace
            and self.service == other.service
            and self.version == other.version
        )
