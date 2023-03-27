#!/usr/bin/env python3
import datetime
import logging
import pytest
from ..client import ArmoniKSubmitter
from typing import Iterator, Optional, List
from .common import DummyChannel
from ..common import TaskOptions, TaskDefinition
from ..protogen.client.submitter_service_pb2_grpc import SubmitterStub
from ..protogen.common.objects_pb2 import Empty, Configuration, Session, TaskIdList, ResultRequest, TaskError, Error, \
    Count, StatusCount
from ..protogen.common.submitter_common_pb2 import CreateSessionRequest, CreateSessionReply, CreateLargeTaskRequest, \
    CreateTaskReply, TaskFilter, ResultReply, AvailabilityReply, WaitRequest
from ..protogen.common.task_status_pb2 import *

logging.basicConfig()
logging.getLogger().setLevel(logging.INFO)


class DummySubmitter(SubmitterStub):
    def __init__(self, channel: DummyChannel, max_chunk_size=300):
        channel.set_instance(self)
        super().__init__(channel)
        self.max_chunk_size = max_chunk_size
        self.large_tasks_requests: List[CreateLargeTaskRequest] = []
        self.task_filter: Optional[TaskFilter] = None
        self.create_session: Optional[CreateSessionRequest] = None
        self.session: Optional[Session] = None
        self.result_stream: List[ResultReply] = []
        self.result_request: Optional[ResultRequest] = None
        self.is_available = True
        self.wait_request: Optional[WaitRequest] = None

    def GetServiceConfiguration(self, _: Empty) -> Configuration:
        return Configuration(data_chunk_max_size=self.max_chunk_size)

    def CreateSession(self, request: CreateSessionRequest) -> CreateSessionReply:
        self.create_session = request
        return CreateSessionReply(session_id="SessionId")

    def CancelSession(self, request: Session) -> Empty:
        self.session = request
        return Empty()

    def CreateLargeTasks(self, request: Iterator[CreateLargeTaskRequest]) -> CreateTaskReply:
        self.large_tasks_requests = [r for r in request]
        return CreateTaskReply(creation_status_list=CreateTaskReply.CreationStatusList(creation_statuses=[
            CreateTaskReply.CreationStatus(
                task_info=CreateTaskReply.TaskInfo(task_id="TaskId", expected_output_keys=["EOK"],
                                                   data_dependencies=["DD"])),
            CreateTaskReply.CreationStatus(error="TestError")]))

    def ListTasks(self, request: TaskFilter) -> TaskIdList:
        self.task_filter = request
        return TaskIdList(task_ids=["TaskId"])

    def TryGetResultStream(self, request: ResultRequest) -> Iterator[ResultReply]:
        self.result_request = request
        for r in self.result_stream:
            yield r

    def WaitForAvailability(self, request: ResultRequest) -> AvailabilityReply:
        self.result_request = request
        return AvailabilityReply(ok=Empty()) if self.is_available else AvailabilityReply(
            error=TaskError(task_id="TaskId", errors=[Error(task_status=TASK_STATUS_ERROR, detail="TestError")]))

    def WaitForCompletion(self, request: WaitRequest) -> Count:
        self.wait_request = request
        return Count(values=[StatusCount(status=TASK_STATUS_COMPLETED, count=1)])


default_task_option = TaskOptions(datetime.timedelta(seconds=300), priority=1, max_retries=5)


@pytest.mark.parametrize("task_options,partitions", [(default_task_option, None), (default_task_option, ["default"])])
def test_armonik_submitter_should_create_session(task_options, partitions):
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    submitter = ArmoniKSubmitter(channel)
    session_id = submitter.create_session(default_task_options=task_options, partition_ids=partitions)
    assert session_id == "SessionId"


def test_armonik_submitter_should_cancel_session():
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    submitter = ArmoniKSubmitter(channel)
    submitter.cancel_session("SessionId")
    assert inner.session is not None
    assert inner.session.id == "SessionId"


def test_armonik_submitter_should_get_config():
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    submitter = ArmoniKSubmitter(channel)
    config = submitter.get_service_configuration()
    assert config is not None
    assert config.data_chunk_max_size == 300


should_submit = [
    [TaskDefinition("Payload1".encode('utf-8'), expected_output_ids=["EOK"], data_dependencies=["DD"]),
     TaskDefinition("Payload2".encode('utf-8'), expected_output_ids=["EOK"], data_dependencies=["DD"])],
    [TaskDefinition("Payload1".encode('utf-8'), expected_output_ids=["EOK"]),
     TaskDefinition("Payload2".encode('utf-8'), expected_output_ids=["EOK"])],
    [TaskDefinition("".encode('utf-8'), expected_output_ids=["EOK"]),
     TaskDefinition("".encode('utf-8'), expected_output_ids=["EOK"])]
]


@pytest.mark.parametrize("task_list,task_options",
                         [(t, default_task_option if i else None) for t in should_submit for i in [True, False]])
def test_armonik_submitter_should_submit(task_list, task_options):
    channel = DummyChannel()
    inner = DummySubmitter(channel, max_chunk_size=5)
    submitter = ArmoniKSubmitter(channel)
    successes, errors = submitter.submit("SessionId", tasks=task_list, task_options=task_options)
    # The dummy submitter has been set to submit one successful task and one submission error
    assert len(successes) == 1
    assert len(errors) == 1
    assert successes[0].id == "TaskId"
    assert successes[0].session_id == "SessionId"
    assert errors[0] == "TestError"

    reqs = inner.large_tasks_requests
    assert len(reqs) > 0
    offset = 0
    assert reqs[0 + offset].WhichOneof("type") == "init_request"
    assert reqs[0 + offset].init_request.session_id == "SessionId"
    assert reqs[1 + offset].WhichOneof("type") == "init_task"
    assert reqs[1 + offset].init_task.header.expected_output_keys[0] == "EOK"
    assert reqs[1 + offset].init_task.header.data_dependencies[0] == "DD" if len(
        task_list[0].data_dependencies) > 0 else len(reqs[1+offset].init_task.header.data_dependencies) == 0
    assert reqs[2 + offset].WhichOneof("type") == "task_payload"
    assert reqs[2 + offset].task_payload.data == "".encode("utf-8") if len(task_list[0].payload) == 0 \
        else reqs[2 + offset].task_payload.data == task_list[0].payload[:5]
    if len(task_list[0].payload) > 0:
        offset += 1
        assert reqs[2 + offset].WhichOneof("type") == "task_payload"
        assert reqs[2 + offset].task_payload.data == task_list[0].payload[5:]
    assert reqs[3 + offset].WhichOneof("type") == "task_payload"
    assert reqs[3 + offset].task_payload.data_complete
    assert reqs[4 + offset].WhichOneof("type") == "init_task"
    assert reqs[4 + offset].init_task.header.expected_output_keys[0] == "EOK"
    assert reqs[4 + offset].init_task.header.data_dependencies[0] == "DD" if len(
        task_list[1].data_dependencies) > 0 else len(reqs[4+offset].init_task.header.data_dependencies) == 0
    assert reqs[5 + offset].WhichOneof("type") == "task_payload"
    assert reqs[5 + offset].task_payload.data == "".encode("utf-8") if len(task_list[1].payload) == 0 \
        else reqs[5 + offset].task_payload.data == task_list[1].payload[:5]
    if len(task_list[1].payload) > 0:
        offset += 1
        assert reqs[5 + offset].WhichOneof("type") == "task_payload"
        assert reqs[5 + offset].task_payload.data == task_list[1].payload[5:]
    assert reqs[6 + offset].WhichOneof("type") == "task_payload"
    assert reqs[6 + offset].task_payload.data_complete
    assert reqs[7 + offset].WhichOneof("type") == "init_task"
    assert reqs[7 + offset].init_task.last_task


