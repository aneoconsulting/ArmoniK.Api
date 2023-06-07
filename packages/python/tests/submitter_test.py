#!/usr/bin/env python3
import datetime
import logging
import pytest
from armonik.client import ArmoniKSubmitter
from typing import Iterator, Optional, List
from .common import DummyChannel
from armonik.common import TaskOptions, TaskDefinition, TaskStatus, timedelta_to_duration
from armonik.protogen.client.submitter_service_pb2_grpc import SubmitterStub
from armonik.protogen.common.objects_pb2 import Empty, Configuration, Session, TaskIdList, ResultRequest, TaskError, Error, \
    Count, StatusCount, DataChunk
from armonik.protogen.common.submitter_common_pb2 import CreateSessionRequest, CreateSessionReply, CreateLargeTaskRequest, \
    CreateTaskReply, TaskFilter, ResultReply, AvailabilityReply, WaitRequest, GetTaskStatusRequest, GetTaskStatusReply

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
        self.get_status_request: Optional[GetTaskStatusRequest] = None

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
        from armonik.protogen.common.task_status_pb2 import TASK_STATUS_ERROR
        self.result_request = request
        return AvailabilityReply(ok=Empty()) if self.is_available else AvailabilityReply(
            error=TaskError(task_id="TaskId", errors=[Error(task_status=TASK_STATUS_ERROR, detail="TestError")]))

    def WaitForCompletion(self, request: WaitRequest) -> Count:
        from armonik.protogen.common.task_status_pb2 import TASK_STATUS_COMPLETED
        self.wait_request = request
        return Count(values=[StatusCount(status=TASK_STATUS_COMPLETED, count=1)])

    def GetTaskStatus(self, request: GetTaskStatusRequest) -> GetTaskStatusReply:
        from armonik.protogen.common.task_status_pb2 import TASK_STATUS_COMPLETED
        self.get_status_request = request
        return GetTaskStatusReply(
            id_statuses=[GetTaskStatusReply.IdStatus(task_id="TaskId", status=TASK_STATUS_COMPLETED)])


default_task_option = TaskOptions(datetime.timedelta(seconds=300), priority=1, max_retries=5)


@pytest.mark.parametrize("task_options,partitions", [(default_task_option, None), (default_task_option, ["default"])])
def test_armonik_submitter_should_create_session(task_options, partitions):
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    submitter = ArmoniKSubmitter(channel)
    session_id = submitter.create_session(default_task_options=task_options, partition_ids=partitions)
    assert session_id == "SessionId"
    assert inner.create_session
    assert inner.create_session.default_task_option.priority == task_options.priority
    assert len(inner.create_session.partition_ids) == 0 if partitions is None else list(inner.create_session.partition_ids) == partitions
    assert len(inner.create_session.default_task_option.options) == len(task_options.options)
    assert inner.create_session.default_task_option.max_duration == timedelta_to_duration(task_options.max_duration)
    assert inner.create_session.default_task_option.max_retries == task_options.max_retries


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
        task_list[0].data_dependencies) > 0 else len(reqs[1 + offset].init_task.header.data_dependencies) == 0
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
        task_list[1].data_dependencies) > 0 else len(reqs[4 + offset].init_task.header.data_dependencies) == 0
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


filters_params = [(session_ids, task_ids, included_statuses, excluded_statuses,
                   (session_ids is None or task_ids is None) and (
                       included_statuses is None or excluded_statuses is None))
                  for session_ids in [["SessionId"], None]
                  for task_ids in [["TaskId"], None]
                  for included_statuses in [[TaskStatus.COMPLETED], None]
                  for excluded_statuses in [[TaskStatus.COMPLETED], None]]


@pytest.mark.parametrize("session_ids,task_ids,included_statuses,excluded_statuses,should_succeed", filters_params)
def test_armonik_submitter_should_list_tasks(session_ids, task_ids, included_statuses, excluded_statuses,
                                             should_succeed):
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    submitter = ArmoniKSubmitter(channel)
    if should_succeed:
        tasks = submitter.list_tasks(session_ids=session_ids, task_ids=task_ids, included_statuses=included_statuses,
                                     excluded_statuses=excluded_statuses)
        assert len(tasks) > 0
        assert tasks[0] == "TaskId"
        assert inner.task_filter is not None
        assert all(map(lambda x: x[1] == session_ids[x[0]], enumerate(inner.task_filter.session.ids)))
        assert all(map(lambda x: x[1] == task_ids[x[0]], enumerate(inner.task_filter.task.ids)))
        assert all(map(lambda x: x[1] == included_statuses[x[0]].value, enumerate(inner.task_filter.included.statuses)))
        assert all(map(lambda x: x[1] == excluded_statuses[x[0]].value, enumerate(inner.task_filter.excluded.statuses)))
    else:
        with pytest.raises(ValueError):
            _ = submitter.list_tasks(session_ids=session_ids, task_ids=task_ids, included_statuses=included_statuses,
                                     excluded_statuses=excluded_statuses)


def test_armonik_submitter_should_get_status():
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    submitter = ArmoniKSubmitter(channel)

    statuses = submitter.get_task_status(["TaskId"])
    assert len(statuses) > 0
    assert "TaskId" in statuses
    assert statuses["TaskId"] == TaskStatus.COMPLETED
    assert inner.get_status_request is not None
    assert len(inner.get_status_request.task_ids) == 1
    assert inner.get_status_request.task_ids[0] == "TaskId"


get_result_should_throw = [
    [],
    [ResultReply(result=DataChunk(data="payload".encode("utf-8")))],
    [ResultReply(result=DataChunk(data="payload".encode("utf-8"))), ResultReply(result=DataChunk(data_complete=True)),
     ResultReply(result=DataChunk(data="payload".encode("utf-8")))],
    [ResultReply(
        error=TaskError(task_id="TaskId", errors=[Error(task_status=TaskStatus.ERROR.value, detail="TestError")]))],
]

get_result_should_succeed = [
    [ResultReply(result=DataChunk(data="payload".encode("utf-8"))), ResultReply(result=DataChunk(data_complete=True))]
]

get_result_should_none = [
    [ResultReply(not_completed_task="NotCompleted")]
]


@pytest.mark.parametrize("stream", [iter(x) for x in get_result_should_succeed])
def test_armonik_submitter_should_get_result(stream):
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    inner.result_stream = stream
    submitter = ArmoniKSubmitter(channel)
    result = submitter.get_result("SessionId", "ResultId")
    assert result is not None
    assert len(result) > 0
    assert inner.result_request
    assert inner.result_request.result_id == "ResultId"
    assert inner.result_request.session == "SessionId"


@pytest.mark.parametrize("stream", [iter(x) for x in get_result_should_throw])
def test_armonik_submitter_get_result_should_throw(stream):
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    inner.result_stream = stream
    submitter = ArmoniKSubmitter(channel)
    with pytest.raises(Exception):
        _ = submitter.get_result("SessionId", "ResultId")


@pytest.mark.parametrize("stream", [iter(x) for x in get_result_should_none])
def test_armonik_submitter_get_result_should_none(stream):
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    inner.result_stream = stream
    submitter = ArmoniKSubmitter(channel)
    result = submitter.get_result("SessionId", "ResultId")
    assert result is None
    assert inner.result_request
    assert inner.result_request.result_id == "ResultId"
    assert inner.result_request.session == "SessionId"


@pytest.mark.parametrize("available", [True, False])
def test_armonik_submitter_wait_availability(available):
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    inner.is_available = available
    submitter = ArmoniKSubmitter(channel)
    reply = submitter.wait_for_availability("SessionId", "ResultId")
    assert reply is not None
    assert reply.is_available() == available
    assert len(reply.errors) == 0 if available else reply.errors[0] == "TestError"


@pytest.mark.parametrize("session_ids,task_ids,included_statuses,excluded_statuses,should_succeed", filters_params)
def test_armonik_submitter_wait_completion(session_ids, task_ids, included_statuses, excluded_statuses, should_succeed):
    channel = DummyChannel()
    inner = DummySubmitter(channel)
    submitter = ArmoniKSubmitter(channel)

    if should_succeed:
        counts = submitter.wait_for_completion(session_ids=session_ids, task_ids=task_ids,
                                               included_statuses=included_statuses,
                                               excluded_statuses=excluded_statuses)
        assert len(counts) > 0
        assert TaskStatus.COMPLETED in counts
        assert counts[TaskStatus.COMPLETED] == 1
        assert inner.wait_request is not None
        assert all(map(lambda x: x[1] == session_ids[x[0]], enumerate(inner.wait_request.filter.session.ids)))
        assert all(map(lambda x: x[1] == task_ids[x[0]], enumerate(inner.wait_request.filter.task.ids)))
        assert all(map(lambda x: x[1] == included_statuses[x[0]].value,
                       enumerate(inner.wait_request.filter.included.statuses)))
        assert all(map(lambda x: x[1] == excluded_statuses[x[0]].value,
                       enumerate(inner.wait_request.filter.excluded.statuses)))
        assert not inner.wait_request.stop_on_first_task_error
        assert not inner.wait_request.stop_on_first_task_cancellation
    else:
        with pytest.raises(ValueError):
            _ = submitter.wait_for_completion(session_ids=session_ids, task_ids=task_ids,
                                              included_statuses=included_statuses,
                                              excluded_statuses=excluded_statuses)
