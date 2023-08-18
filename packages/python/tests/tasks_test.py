#!/usr/bin/env python3
from typing import Optional

from datetime import datetime
from .common import DummyChannel
from armonik.client import ArmoniKTasks
from armonik.common import TaskStatus, datetime_to_timestamp, Task
from armonik.protogen.client.tasks_service_pb2_grpc import TasksStub
from armonik.protogen.common.tasks_common_pb2 import GetTaskRequest, GetTaskResponse, TaskDetailed
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
                         status=TaskStatus.COMPLETED.value, status_message="Message",
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
