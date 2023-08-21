from dataclasses import dataclass, field
from datetime import timedelta, datetime
from typing import Optional, List, Dict

from ..protogen.common.tasks_common_pb2 import TaskDetailed
from .helpers import duration_to_timedelta, timedelta_to_duration, timestamp_to_datetime
from ..protogen.common.objects_pb2 import Empty, Output as WorkerOutput, TaskOptions as RawTaskOptions
from .enumwrapper import TaskStatus


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
        return cls(max_duration=duration_to_timedelta(task_options.max_duration),
                   max_retries=task_options.max_retries,
                   priority=task_options.priority,
                   partition_id=task_options.partition_id,
                   application_name=task_options.application_name,
                   application_version=task_options.application_version,
                   application_namespace=task_options.application_namespace,
                   application_service=task_options.application_service,
                   engine_type=task_options.engine_type,
                   options=task_options.options
                   )

    def to_message(self) -> RawTaskOptions:
        return RawTaskOptions(max_duration=timedelta_to_duration(self.max_duration),
                              max_retries=self.max_retries,
                              priority=self.priority,
                              partition_id=self.partition_id,
                              application_name=self.application_name,
                              application_version=self.application_version,
                              application_namespace=self.application_namespace,
                              application_service=self.application_service,
                              engine_type=self.engine_type,
                              options=self.options
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
    payload: bytes
    expected_output_ids: List[str] = field(default_factory=list)
    data_dependencies: List[str] = field(default_factory=list)

    def __post_init__(self):
        if len(self.expected_output_ids) <= 0:
            raise ValueError("expected_output_ids must be not be empty")


@dataclass()
class Task:
    id: Optional[str] = None
    session_id: Optional[str] = None
    owner_pod_id: Optional[str] = None
    parent_task_ids: List[str] = field(default_factory=list)
    data_dependencies: List[str] = field(default_factory=list)
    expected_output_ids: List[str] = field(default_factory=list)
    retry_of_ids: List[str] = field(default_factory=list)
    status: TaskStatus = TaskStatus.UNSPECIFIED
    status_message: Optional[str] = None
    options: Optional[TaskOptions] = None
    created_at: Optional[datetime] = None
    submitted_at: Optional[datetime] = None
    started_at: Optional[datetime] = None
    ended_at: Optional[datetime] = None
    pod_ttl: Optional[datetime] = None
    output: Optional[Output] = None
    pod_hostname: Optional[str] = None
    received_at: Optional[datetime] = None
    acquired_at: Optional[datetime] = None

    def refresh(self, task_client) -> None:
        """Refresh the fields of this task object by using the given task client

        Args:
            task_client: ArmoniKTasks client
        """
        result : "Task" = task_client.get_task(self.id)
        self.session_id = result.session_id
        self.owner_pod_id = result.owner_pod_id
        self.parent_task_ids = result.parent_task_ids
        self.data_dependencies = result.data_dependencies
        self.expected_output_ids = result.expected_output_ids
        self.retry_of_ids = result.retry_of_ids
        self.status = TaskStatus(result.status)
        self.status_message = result.status_message
        self.options = result.options
        self.created_at = result.created_at
        self.submitted_at = result.submitted_at
        self.started_at = result.started_at
        self.ended_at = result.ended_at
        self.pod_ttl = result.pod_ttl
        self.output = result.output
        self.pod_hostname = result.pod_hostname
        self.received_at = result.received_at
        self.acquired_at = result.acquired_at
        self.is_init = True

    @classmethod
    def from_message(cls, task_raw: TaskDetailed) -> "Task":
        return cls(
            id=task_raw.id,
            session_id=task_raw.session_id,
            owner_pod_id=task_raw.owner_pod_id,
            parent_task_ids=list(task_raw.parent_task_ids),
            data_dependencies=list(task_raw.data_dependencies),
            expected_output_ids=list(task_raw.expected_output_ids),
            retry_of_ids=list(task_raw.retry_of_ids),
            status=TaskStatus(task_raw.status),
            status_message=task_raw.status_message,
            options=TaskOptions.from_message(task_raw.options),
            created_at=timestamp_to_datetime(task_raw.created_at),
            submitted_at=timestamp_to_datetime(task_raw.submitted_at),
            started_at=timestamp_to_datetime(task_raw.started_at),
            ended_at=timestamp_to_datetime(task_raw.ended_at),
            pod_ttl=timestamp_to_datetime(task_raw.pod_ttl),
            output=Output(
                error=(task_raw.output.error if not task_raw.output.success else None)),
            pod_hostname=task_raw.pod_hostname,
            received_at=timestamp_to_datetime(task_raw.received_at),
            acquired_at=timestamp_to_datetime(task_raw.acquired_at)
        )


@dataclass()
class ResultAvailability:
    errors: List[str] = field(default_factory=list)

    def is_available(self) -> bool:
        return len(self.errors) == 0
