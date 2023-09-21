from grpc import Channel
from typing import Type
import enum

from ..common import Task
from ..common.filter import StringFilter, FilterDisjunction, FilterConjunction, StatusFilter, DateFilter
from ..protogen.client.tasks_service_pb2_grpc import TasksStub
from ..protogen.common.tasks_common_pb2 import GetTaskRequest, GetTaskResponse
from ..protogen.common.tasks_filters_pb2 import Filters as rawFilters, FiltersAnd as rawFilterAnd, FilterField as rawFilterField, FilterStatus as rawFilterStatus

from ..protogen.common.tasks_fields_pb2 import *


class TaskFilterAnd(FilterConjunction):
    def disjunction_type(self) -> Type["FilterDisjunction"]:
        return TaskFilter

    def message_type(self) -> Type:
        return rawFilterAnd


class TaskFilter(FilterDisjunction):
    def message_type(self) -> Type:
        return rawFilters

    def conjunction_type(self) -> Type["FilterConjunction"]:
        return TaskFilterAnd


class TaskFieldFilter(enum.Enum):
    TASK_ID = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_TASK_ID)), TaskFilterAnd, rawFilterField)
    SESSION_ID = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_SESSION_ID)), TaskFilterAnd, rawFilterField)
    OWNER_POD_ID = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_OWNER_POD_ID)), TaskFilterAnd, rawFilterField)
    INITIAL_TASK_ID = StringFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID)), TaskFilterAnd, rawFilterField)
    STATUS = StatusFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_STATUS)), TaskFilterAnd, rawFilterField, rawFilterStatus)
    CREATED_AT = DateFilter(TaskField(task_summary_field=TaskSummaryField(field=TASK_SUMMARY_ENUM_FIELD_CREATED_AT)), TaskFilterAnd, rawFilterField)



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
