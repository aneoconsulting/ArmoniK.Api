#!/usr/bin/env python3
import dataclasses
from typing import Optional, List, Any, Union
from google.protobuf.timestamp_pb2 import Timestamp

from datetime import datetime

import pytest

from .common import DummyChannel
from armonik.client import ArmoniKTasks
from armonik.client.tasks import TaskFieldFilter
from armonik.common import TaskStatus, datetime_to_timestamp, Task
from armonik.common.filter import StringFilter, FilterDisjunction, FilterConjunction, SimpleFilter
from armonik.protogen.client.tasks_service_pb2_grpc import TasksStub
from armonik.protogen.common.tasks_common_pb2 import GetTaskRequest, GetTaskResponse, TaskDetailed
from armonik.protogen.common.tasks_filters_pb2 import Filters, FilterField
from armonik.protogen.common.filters_common_pb2 import *
from armonik.protogen.common.tasks_fields_pb2 import *
from .submitter_test import default_task_option


class DummyTasksService(TasksStub):
    def __init__(self, channel: DummyChannel):
        channel.set_instance(self)
        super().__init__(channel)
        self.task_request: Optional[GetTaskRequest] = None

    def GetTask(self, request: GetTaskRequest) -> GetTaskResponse:
        self.task_request = request
        raw = TaskDetailed(id="TaskId", session_id="SessionId", owner_pod_id="PodId", parent_task_ids=["ParentTaskId"],
                         data_dependencies=["DD"], expected_output_ids=["EOK"], retry_of_ids=["RetryId"],
                         status=TaskStatus.COMPLETED, status_message="Message",
                         options=default_task_option.to_message(),
                        created_at=datetime_to_timestamp(datetime.now()),
                         started_at=datetime_to_timestamp(datetime.now()),
                         submitted_at=datetime_to_timestamp(datetime.now()),
                         ended_at=datetime_to_timestamp(datetime.now()), pod_ttl=datetime_to_timestamp(datetime.now()),
                         output=TaskDetailed.Output(success=True), pod_hostname="Hostname", received_at=datetime_to_timestamp(datetime.now()),
                         acquired_at=datetime_to_timestamp(datetime.now())
         )
        return GetTaskResponse(task=raw)


def test_tasks_get_task_should_succeed():
    channel = DummyChannel()
    inner = DummyTasksService(channel)
    tasks = ArmoniKTasks(channel)
    task = tasks.get_task("TaskId")
    assert task is not None
    assert inner.task_request is not None
    assert inner.task_request.task_id == "TaskId"
    assert task.id == "TaskId"
    assert task.session_id == "SessionId"
    assert task.parent_task_ids == ["ParentTaskId"]
    assert task.output
    assert task.output.success


def test_task_refresh():
    channel = DummyChannel()
    inner = DummyTasksService(channel)
    tasks = ArmoniKTasks(channel)
    current = Task(id="TaskId")
    current.refresh(tasks)
    assert current is not None
    assert inner.task_request is not None
    assert inner.task_request.task_id == "TaskId"
    assert current.id == "TaskId"
    assert current.session_id == "SessionId"
    assert current.parent_task_ids == ["ParentTaskId"]
    assert current.output
    assert current.output.success


def test_task_filters():
    filt: StringFilter = TaskFieldFilter.TASK_ID == "TaskId"
    message = filt.to_message()
    assert isinstance(message, FilterField)
    assert message.field.WhichOneof("field") == "task_summary_field"
    assert message.field.task_summary_field.field == TASK_SUMMARY_ENUM_FIELD_TASK_ID
    assert message.filter_string.value == "TaskId"
    assert message.filter_string.operator == FILTER_STRING_OPERATOR_EQUAL

    filt: StringFilter = TaskFieldFilter.TASK_ID != "TaskId"
    message = filt.to_message()
    assert isinstance(message, FilterField)
    assert message.field.WhichOneof("field") == "task_summary_field"
    assert message.field.task_summary_field.field == TASK_SUMMARY_ENUM_FIELD_TASK_ID
    assert message.filter_string.value == "TaskId"
    assert message.filter_string.operator == FILTER_STRING_OPERATOR_NOT_EQUAL


@dataclasses.dataclass
class SimpleFieldFilter:
    field: Any
    value: Any
    operator: Any


@pytest.mark.parametrize("filt,n_or,n_and,filters", [
    (
        (TaskFieldFilter.INITIAL_TASK_ID == "TestId"),
        1, [1],
        [
            SimpleFieldFilter(TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID, "TestId", FILTER_STRING_OPERATOR_EQUAL)
        ]
    ),
    (
        (TaskFieldFilter.APPLICATION_NAME.contains("TestName") & (TaskFieldFilter.CREATED_AT > Timestamp(seconds=1000, nanos=500))),
        1, [2],
        [
            SimpleFieldFilter(TASK_OPTION_ENUM_FIELD_APPLICATION_NAME, "TestName", FILTER_STRING_OPERATOR_CONTAINS),
            SimpleFieldFilter(TASK_SUMMARY_ENUM_FIELD_CREATED_AT, Timestamp(seconds=1000, nanos=500), FILTER_DATE_OPERATOR_AFTER)
        ]
    ),
    (
        (((TaskFieldFilter.MAX_RETRIES <= 3) & ~(TaskFieldFilter.SESSION_ID == "SessionId")) | (TaskFieldFilter.task_options_key("MyKey").startswith("Start"))),
        2, [1, 2],
        [
            SimpleFieldFilter(TASK_OPTION_ENUM_FIELD_MAX_RETRIES, 3, FILTER_NUMBER_OPERATOR_LESS_THAN_OR_EQUAL),
            SimpleFieldFilter(TASK_SUMMARY_ENUM_FIELD_SESSION_ID, "SessionId", FILTER_STRING_OPERATOR_NOT_EQUAL),
            SimpleFieldFilter("MyKey", "Start", FILTER_STRING_OPERATOR_STARTS_WITH)
        ]
    ),
    (
        (((TaskFieldFilter.PRIORITY > 3) & ~(TaskFieldFilter.STATUS == TaskStatus.COMPLETED) & TaskFieldFilter.APPLICATION_VERSION.contains("1.0")) | (TaskFieldFilter.ENGINE_TYPE.endswith("Test") & (TaskFieldFilter.ENDED_AT <= Timestamp(seconds=1000, nanos=500)))),
        2, [2, 3],
        [
            SimpleFieldFilter(TASK_OPTION_ENUM_FIELD_PRIORITY, 3, FILTER_NUMBER_OPERATOR_GREATER_THAN),
            SimpleFieldFilter(TASK_SUMMARY_ENUM_FIELD_STATUS, TaskStatus.COMPLETED, FILTER_STATUS_OPERATOR_NOT_EQUAL),
            SimpleFieldFilter(TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION, "1.0", FILTER_STRING_OPERATOR_CONTAINS),
            SimpleFieldFilter(TASK_OPTION_ENUM_FIELD_ENGINE_TYPE, "Test", FILTER_STRING_OPERATOR_ENDS_WITH),
            SimpleFieldFilter(TASK_SUMMARY_ENUM_FIELD_ENDED_AT, Timestamp(seconds=1000, nanos=500), FILTER_DATE_OPERATOR_BEFORE_OR_EQUAL),
        ]
    ),
    (
        (((TaskFieldFilter.PRIORITY >= 3) * -(TaskFieldFilter.STATUS != TaskStatus.COMPLETED) * -TaskFieldFilter.APPLICATION_VERSION.contains("1.0")) + (TaskFieldFilter.ENGINE_TYPE.endswith("Test") * (TaskFieldFilter.ENDED_AT <= Timestamp(seconds=1000, nanos=500)))),
        2, [2, 3],
        [
            SimpleFieldFilter(TASK_OPTION_ENUM_FIELD_PRIORITY, 3, FILTER_NUMBER_OPERATOR_GREATER_THAN_OR_EQUAL),
            SimpleFieldFilter(TASK_SUMMARY_ENUM_FIELD_STATUS, TaskStatus.COMPLETED, FILTER_STATUS_OPERATOR_EQUAL),
            SimpleFieldFilter(TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION, "1.0", FILTER_STRING_OPERATOR_NOT_CONTAINS),
            SimpleFieldFilter(TASK_OPTION_ENUM_FIELD_ENGINE_TYPE, "Test", FILTER_STRING_OPERATOR_ENDS_WITH),
            SimpleFieldFilter(TASK_SUMMARY_ENUM_FIELD_ENDED_AT, Timestamp(seconds=1000, nanos=500), FILTER_DATE_OPERATOR_BEFORE_OR_EQUAL),
        ]
    )
])
def test_filter_combination(filt: Union[SimpleFilter, FilterConjunction, FilterDisjunction], n_or: int, n_and: List[int], filters: List[SimpleFieldFilter]):
    filt = filt.to_disjunction()
    assert len(filt.filters) == n_or
    sorted_n_and = sorted(n_and)
    sorted_actual = sorted([len(f.filters) for f in filt.filters])
    assert len(sorted_n_and) == len(sorted_actual)
    assert all((sorted_n_and[i] == sorted_actual[i] for i in range(len(sorted_actual))))
    for f in filt.filters:
        for ff in f.filters:
            field_value = getattr(ff.field, ff.field.WhichOneof("field")).field
            for i, expected in enumerate(filters):
                if expected.field == field_value and expected.value == ff.value and expected.operator == ff.operator:
                    filters.pop(i)
                    break
            else:
                print(f"Could not find {str(ff)}")
                assert False
    assert len(filters) == 0

def test_name_from_value():
    assert TaskStatus.name_from_value(TaskStatus.COMPLETED) == "TASK_STATUS_COMPLETED"
