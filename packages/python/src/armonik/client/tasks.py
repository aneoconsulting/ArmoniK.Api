from __future__ import annotations

from typing import Dict, List, Optional, Tuple

from deprecation import deprecated
from grpc import Channel

from .. import __version__
from ..common import Direction, Task, TaskDefinition, TaskOptions, TaskStatus
from ..common.filter import Filter, StringFilter, TaskFilter
from ..common.helpers import batched
from ..protogen.client.tasks_service_pb2_grpc import TasksStub
from ..protogen.common.sort_direction_pb2 import SortDirection
from ..protogen.common.tasks_common_pb2 import (
    CancelTasksRequest,
    CountTasksByStatusRequest,
    CountTasksByStatusResponse,
    GetResultIdsRequest,
    GetResultIdsResponse,
    GetTaskRequest,
    GetTaskResponse,
    ListTasksDetailedResponse,
    ListTasksRequest,
    ListTasksResponse,
    SubmitTasksRequest,
)


@deprecated("3.19.0", None, __version__, "Use Task.<name of the field> instead")
class TaskFieldFilter:
    """
    Enumeration of the available filters
    """

    TASK_ID = Task.id
    SESSION_ID = Task.session_id
    OWNER_POD_ID = Task.owner_pod_id
    INITIAL_TASK_ID = Task.initial_task_id
    STATUS = Task.status
    CREATED_AT = Task.created_at
    SUBMITTED_AT = Task.submitted_at
    STARTED_AT = Task.started_at
    ENDED_AT = Task.ended_at
    CREATION_TO_END_DURATION = Task.creation_to_end_duration
    PROCESSING_TO_END_DURATION = Task.processing_to_end_duration
    POD_TTL = Task.pod_ttl
    POD_HOSTNAME = Task.pod_hostname
    RECEIVED_AT = Task.received_at
    ACQUIRED_AT = Task.acquired_at
    ERROR = Task.output.error

    MAX_DURATION = Task.options.max_duration
    MAX_RETRIES = Task.options.max_retries
    PRIORITY = Task.options.priority
    PARTITION_ID = Task.options.partition_id
    APPLICATION_NAME = Task.options.application_name
    APPLICATION_VERSION = Task.options.application_version
    APPLICATION_NAMESPACE = Task.options.application_namespace
    APPLICATION_SERVICE = Task.options.application_service
    ENGINE_TYPE = Task.options.engine_type

    @staticmethod
    def task_options_key(option_key: str) -> StringFilter:
        """
        Filter for the TaskOptions.Options dictionary
        Args:
            option_key: key in the dictionary

        Returns:
            Corresponding filter
        """
        return Task.options[option_key]


class ArmoniKTasks:
    def __init__(self, grpc_channel: Channel):
        """Tasks service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = TasksStub(grpc_channel)

    def get_task(self, task_id: str) -> Task:
        """Get task information from task id

        Args:
            task_id: Id of the task

        Returns:
            Task object with the information
        """
        task_response: GetTaskResponse = self._client.GetTask(GetTaskRequest(task_id=task_id))
        return Task.from_message(task_response.task)

    def list_tasks(
        self,
        task_filter: Optional[Filter] = None,
        with_errors: bool = False,
        page: int = 0,
        page_size: int = 1000,
        sort_field: Filter = Task.id,
        sort_direction: SortDirection = Direction.ASC,
        detailed: bool = True,
    ) -> Tuple[int, List[Task]]:
        """List tasks

        If the total returned exceeds the requested page size, you may want to use this function again and ask for subsequent pages.

        Args:
            task_filter: Filter for the tasks to be listed
            with_errors: Retrieve the error if the task had errors, defaults to false
            page: page number to request, this can be useful when paginating the result, defaults to 0
            page_size: size of a page, defaults to 1000
            sort_field: field on which to sort the resulting list, defaults to the task_id
            sort_direction: direction of the sort, defaults to ascending
            detailed: Whether to retrieve the detailed description of the task.

        Returns:
            A tuple containing :
            - The total number of tasks for the given filter
            - The obtained list of tasks
        """
        request = ListTasksRequest(
            page=page,
            page_size=page_size,
            filters=(
                TaskFilter().to_message() if task_filter is None else task_filter.to_message()
            ),
            sort=ListTasksRequest.Sort(field=sort_field.field, direction=sort_direction),
            with_errors=with_errors,
        )
        if detailed:
            response: ListTasksDetailedResponse = self._client.ListTasksDetailed(request)
            return response.total, [Task.from_message(t) for t in response.tasks]
        response: ListTasksResponse = self._client.ListTasks(request)
        return response.total, [Task.from_summary(t) for t in response.tasks]

    def cancel_tasks(self, task_ids: List[str], chunk_size: Optional[int] = 500):
        """Cancel tasks.

        Args:
            task_ids: IDs of the tasks.
            chunk_size: Batch size for cancelling.

        Return:
            The list of cancelled tasks.
        """
        for task_id_batch in batched(task_ids, chunk_size):
            request = CancelTasksRequest(task_ids=task_id_batch)
            self._client.CancelTasks(request)

    def get_result_ids(
        self, task_ids: List[str], chunk_size: Optional[int] = 500
    ) -> Dict[str, List[str]]:
        """Get result IDs of a list of tasks.

        Args:
            task_ids: The IDs of the tasks.
            chunk_size: Batch size for retrieval.

        Return:
            A dictionary mapping the ID of a task to the IDs of its results..
        """
        tasks_result_ids = {}

        for task_ids_batch in batched(task_ids, chunk_size):
            request = GetResultIdsRequest(task_id=task_ids_batch)
            result_ids_response: GetResultIdsResponse = self._client.GetResultIds(request)
            for t in result_ids_response.task_results:
                tasks_result_ids[t.task_id] = list(t.result_ids)
        return tasks_result_ids

    def count_tasks_by_status(self, task_filter: Optional[Filter] = None) -> Dict[TaskStatus, int]:
        """Get number of tasks by status.

        Args:
            task_filter: Filter for the tasks to be listed

        Return:
            A dictionnary mapping each status to the number of filtered tasks.
        """
        request = CountTasksByStatusRequest(
            filters=(TaskFilter().to_message() if task_filter is None else task_filter.to_message())
        )
        count_tasks_by_status_response: CountTasksByStatusResponse = (
            self._client.CountTasksByStatus(request)
        )
        return {
            TaskStatus(status_count.status): status_count.count
            for status_count in count_tasks_by_status_response.status
        }

    def submit_tasks(
        self,
        session_id: str,
        tasks: List[TaskDefinition],
        default_task_options: Optional[TaskOptions] = None,
        chunk_size: Optional[int] = 100,
    ) -> List[Task]:
        """Submit tasks to ArmoniK.

        Args:
            session_id: Session Id
            tasks: List of task definitions
            default_task_options: Default Task Options used if a task has its options not set
            chunk_size: Batch size for submission

        Returns:
            List of successfully sent tasks
        """
        submitted = []
        for tasks_batch in batched(tasks, chunk_size):
            task_creations = []

            for t in tasks_batch:
                task_creation = SubmitTasksRequest.TaskCreation(
                    expected_output_keys=t.expected_output_ids,
                    payload_id=t.payload_id,
                    data_dependencies=t.data_dependencies,
                    task_options=t.options.to_message() if t.options else None,
                )
                task_creations.append(task_creation)

            request = SubmitTasksRequest(
                session_id=session_id,
                task_creations=task_creations,
                task_options=(default_task_options.to_message() if default_task_options else None),
            )
            submitted.extend(
                Task(
                    id=t.task_id,
                    session_id=session_id,
                    expected_output_ids=list(t.expected_output_ids),
                    data_dependencies=list(t.data_dependencies),
                    payload_id=t.payload_id,
                )
                for t in self._client.SubmitTasks(request).task_infos
            )
        return submitted
