from __future__ import annotations
from grpc import Channel
from typing import cast, Dict, Optional, Tuple, List

from ..common import Task, Direction, TaskDefinition, TaskOptions, TaskStatus
from ..common.filter import StringFilter, StatusFilter, DateFilter, NumberFilter, Filter, DurationFilter
from ..protogen.client.tasks_service_pb2_grpc import TasksStub
from ..protogen.common.tasks_common_pb2 import GetTaskRequest, GetTaskResponse, ListTasksRequest, ListTasksDetailedResponse, CancelTasksRequest, CancelTasksResponse, GetResultIdsRequest, GetResultIdsResponse, SubmitTasksRequest, SubmitTasksResponse, CountTasksByStatusRequest, CountTasksByStatusResponse, ListTasksResponse
from ..protogen.common.tasks_filters_pb2 import Filters as rawFilters, FiltersAnd as rawFilterAnd, FilterField as rawFilterField, FilterStatus as rawFilterStatus
from ..protogen.common.sort_direction_pb2 import SortDirection
from ..protogen.common.tasks_fields_pb2 import *
from ..common.helpers import batched


class TaskFieldFilter:
    """
    Enumeration of the available filters
    """
    TASK_ID = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_TASK_ID)), rawFilters, rawFilterAnd, rawFilterField)
    SESSION_ID = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_SESSION_ID)), rawFilters, rawFilterAnd, rawFilterField)
    OWNER_POD_ID = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_OWNER_POD_ID)), rawFilters, rawFilterAnd, rawFilterField)
    INITIAL_TASK_ID = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID)), rawFilters, rawFilterAnd, rawFilterField)
    STATUS = StatusFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_STATUS)), rawFilters, rawFilterAnd, rawFilterField, rawFilterStatus)
    CREATED_AT = DateFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_CREATED_AT)), rawFilters, rawFilterAnd, rawFilterField)
    SUBMITTED_AT = DateFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_SUBMITTED_AT)), rawFilters, rawFilterAnd, rawFilterField)
    STARTED_AT = DateFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_STARTED_AT)), rawFilters, rawFilterAnd, rawFilterField)
    ENDED_AT = DateFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_ENDED_AT)), rawFilters, rawFilterAnd, rawFilterField)
    CREATION_TO_END_DURATION = DurationFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_CREATION_TO_END_DURATION)), rawFilters, rawFilterAnd, rawFilterField)
    PROCESSING_TO_END_DURATION = DurationFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_PROCESSING_TO_END_DURATION)), rawFilters, rawFilterAnd, rawFilterField)
    POD_TTL = DateFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_POD_TTL)), rawFilters, rawFilterAnd, rawFilterField)
    POD_HOSTNAME = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_POD_HOSTNAME)), rawFilters, rawFilterAnd, rawFilterField)
    RECEIVED_AT = DateFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_RECEIVED_AT)), rawFilters, rawFilterAnd, rawFilterField)
    ACQUIRED_AT = DateFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_ACQUIRED_AT)), rawFilters, rawFilterAnd, rawFilterField)
    ERROR = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_ERROR)), rawFilters, rawFilterAnd, rawFilterField)

    MAX_DURATION = DurationFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_MAX_DURATION)), rawFilters, rawFilterAnd, rawFilterField)
    MAX_RETRIES = NumberFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_MAX_RETRIES)), rawFilters, rawFilterAnd, rawFilterField)
    PRIORITY = NumberFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_PRIORITY)), rawFilters, rawFilterAnd, rawFilterField)
    PARTITION_ID = StringFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_PARTITION_ID)), rawFilters, rawFilterAnd, rawFilterField)
    APPLICATION_NAME = StringFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_APPLICATION_NAME)), rawFilters, rawFilterAnd, rawFilterField)
    APPLICATION_VERSION = StringFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION)), rawFilters, rawFilterAnd, rawFilterField)
    APPLICATION_NAMESPACE = StringFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE)), rawFilters, rawFilterAnd, rawFilterField)
    APPLICATION_SERVICE = StringFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE)), rawFilters, rawFilterAnd, rawFilterField)
    ENGINE_TYPE = StringFilter(TaskField(task_option_field=TaskOptionField(field=TASK_OPTION_ENUM_FIELD_ENGINE_TYPE)), rawFilters, rawFilterAnd, rawFilterField)

    @staticmethod
    def task_options_key(option_key: str) -> StringFilter:
        """
        Filter for the TaskOptions.Options dictionary
        Args:
            option_key: key in the dictionary

        Returns:
            Corresponding filter
        """
        return StringFilter(TaskField(task_option_generic_field=TaskOptionGenericField(field=option_key)), rawFilters, rawFilterAnd, rawFilterField)


class ArmoniKTasks:
    def __init__(self, grpc_channel: Channel):
        """ Tasks service client

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

    def list_tasks(self, task_filter: Filter | None = None, with_errors: bool = False, page: int = 0, page_size: int = 1000, sort_field: Filter = TaskFieldFilter.TASK_ID, sort_direction: SortDirection = Direction.ASC, detailed: bool = True) -> Tuple[int, List[Task]]:
        """List tasks

        If the total returned exceeds the requested page size, you may want to use this function again and ask for subsequent pages.

        Args:
            task_filter: Filter for the tasks to be listed
            with_errors: Retrieve the error if the task had errors, defaults to false
            page: page number to request, this can be useful when paginating the result, defaults to 0
            page_size: size of a page, defaults to 1000
            sort_field: field on which to sort the resulting list, defaults to the task_id
            sort_direction: direction of the sort, defaults to ascending
            detailed: Wether to retrieve the detailed description of the task.

        Returns:
            A tuple containing :
            - The total number of tasks for the given filter
            - The obtained list of tasks
        """
        request = ListTasksRequest(page=page,
                                    page_size=page_size,
                                    filters=cast(rawFilters, task_filter.to_disjunction().to_message()) if task_filter else None,
                                    sort=ListTasksRequest.Sort(field=cast(TaskField, sort_field.field), direction=sort_direction),
                                    with_errors=with_errors
                                )
        if detailed:
            response: ListTasksDetailedResponse = self._client.ListTasksDetailed(request)
            return response.total, [Task.from_message(t) for t in response.tasks]
        response: ListTasksResponse = self._client.ListTasks(request)
        return response.total, [Task.from_message(t) for t in response.tasks]

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

    def get_result_ids(self, task_ids: List[str], chunk_size: Optional[int] = 500) -> Dict[str, List[str]]:
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

    def count_tasks_by_status(self, task_filter: Filter | None = None) -> Dict[TaskStatus, int]:
        """Get number of tasks by status.

        Args:
            task_filter: Filter for the tasks to be listed
        
        Return:
            A dictionnary mapping each status to the number of filtered tasks.
        """
        request = CountTasksByStatusRequest(
            filters=cast(rawFilters, task_filter.to_disjunction().to_message()) if task_filter else None
        )
        count_tasks_by_status_response: CountTasksByStatusResponse = self._client.CountTasksByStatus(request)
        return {TaskStatus(status_count.status): status_count.count for status_count in count_tasks_by_status_response.status}

    def submit_tasks(self, session_id: str, tasks: List[TaskDefinition], default_task_options: Optional[TaskOptions | None] = None, chunk_size: Optional[int] = 100) -> List[Task]:
        """Submit tasks to ArmoniK.

        Args:
            session_id: Session Id
            tasks: List of task definitions
            default_task_options: Default Task Options used if a task has its options not set
            chunk_size: Batch size for submission

        Returns:
            Tuple containing the list of successfully sent tasks, and
            the list of submission errors if any
        """
        for tasks_batch in batched(tasks, chunk_size):
            task_creations = []

            for t in tasks_batch:
                task_creation = SubmitTasksRequest.TaskCreation(
                    expected_output_keys=t.expected_output_ids,
                    payload_id=t.payload_id,
                    data_dependencies=t.data_dependencies,
                    task_options=t.options.to_message() if t.options else None
                )
                task_creations.append(task_creation)

            request = SubmitTasksRequest(
                session_id=session_id,
                task_creations=task_creations,
                task_options=default_task_options.to_message() if default_task_options else None
            )

            self._client.SubmitTasks(request)
