#!/usr/bin/env python3
import pytest
from typing import Iterator
from .common import DummyChannel
from armonik.common import TaskDefinition
from armonik.worker import TaskHandler
from armonik.protogen.worker.agent_service_pb2_grpc import AgentStub
from armonik.protogen.common.agent_common_pb2 import CreateTaskRequest, CreateTaskReply, Result, ResultReply
from armonik.protogen.common.worker_common_pb2 import ProcessRequest
from armonik.protogen.common.objects_pb2 import Configuration, DataChunk
import logging

logging.basicConfig()
logging.getLogger().setLevel(logging.INFO)


class DummyAgent(AgentStub):

    def __init__(self, channel: DummyChannel) -> None:
        channel.set_instance(self)
        super(DummyAgent, self).__init__(channel)
        self.create_task_messages = []
        self.send_result_task_message = []

    def CreateTask(self, request_iterator: Iterator[CreateTaskRequest]) -> CreateTaskReply:
        self.create_task_messages = [r for r in request_iterator]
        return CreateTaskReply(creation_status_list=CreateTaskReply.CreationStatusList(creation_statuses=[
            CreateTaskReply.CreationStatus(
                task_info=CreateTaskReply.TaskInfo(task_id="TaskId", expected_output_keys=["EOK"],
                                                   data_dependencies=["DD"]))]))

    def SendResult(self, request_iterator: Iterator[Result]) -> ResultReply:
        self.send_result_task_message = [r for r in request_iterator]
        return ResultReply()


class Reqs:
    InitData1 = ProcessRequest(communication_token="Token",
                               compute=ProcessRequest.ComputeRequest(
                                   init_data=ProcessRequest.ComputeRequest.InitData(key="DataKey1")))
    InitData2 = ProcessRequest(communication_token="Token",
                               compute=ProcessRequest.ComputeRequest(
                                   init_data=ProcessRequest.ComputeRequest.InitData(key="DataKey2")))
    LastDataTrue = ProcessRequest(communication_token="Token",
                                  compute=ProcessRequest.ComputeRequest(
                                      init_data=ProcessRequest.ComputeRequest.InitData(last_data=True)))
    LastDataFalse = ProcessRequest(communication_token="Token",
                                   compute=ProcessRequest.ComputeRequest(
                                       init_data=ProcessRequest.ComputeRequest.InitData(last_data=False)))
    InitRequestPayload = ProcessRequest(communication_token="Token",
                                        compute=ProcessRequest.ComputeRequest(
                                            init_request=ProcessRequest.ComputeRequest.InitRequest(
                                                payload=DataChunk(data="test".encode("utf-8")),
                                                configuration=Configuration(data_chunk_max_size=100),
                                                expected_output_keys=["EOK"], session_id="SessionId",
                                                task_id="TaskId")))
    InitRequestEmptyPayload = ProcessRequest(communication_token="Token",
                                             compute=ProcessRequest.ComputeRequest(
                                                 init_request=ProcessRequest.ComputeRequest.InitRequest(
                                                     configuration=Configuration(data_chunk_max_size=100),
                                                     expected_output_keys=["EOK"], session_id="SessionId",
                                                     task_id="TaskId")))
    Payload1 = ProcessRequest(communication_token="Token",
                              compute=ProcessRequest.ComputeRequest(
                                  payload=DataChunk(data="Payload1".encode("utf-8"))))
    Payload2 = ProcessRequest(communication_token="Token",
                              compute=ProcessRequest.ComputeRequest(
                                  payload=DataChunk(data="Payload2".encode("utf-8"))))
    PayloadComplete = ProcessRequest(communication_token="Token",
                                     compute=ProcessRequest.ComputeRequest(payload=DataChunk(data_complete=True)))
    DataComplete = ProcessRequest(communication_token="Token",
                                  compute=ProcessRequest.ComputeRequest(data=DataChunk(data_complete=True)))
    Data1 = ProcessRequest(communication_token="Token",
                           compute=ProcessRequest.ComputeRequest(
                               data=DataChunk(data="Data1".encode("utf-8"))))
    Data2 = ProcessRequest(communication_token="Token",
                           compute=ProcessRequest.ComputeRequest(
                               data=DataChunk(data="Data2".encode("utf-8"))))


should_throw_cases = [
    [],
    [Reqs.InitData1],
    [Reqs.InitData2],
    [Reqs.LastDataTrue],
    [Reqs.LastDataFalse],
    [Reqs.InitRequestPayload],
    [Reqs.DataComplete],
    [Reqs.InitRequestEmptyPayload],
    [Reqs.InitRequestPayload, Reqs.PayloadComplete, Reqs.InitData1, Reqs.Data1, Reqs.LastDataTrue],
    [Reqs.InitRequestPayload, Reqs.InitData1, Reqs.Data1, Reqs.DataComplete, Reqs.LastDataTrue],
    [Reqs.InitRequestPayload, Reqs.PayloadComplete, Reqs.Data1, Reqs.DataComplete, Reqs.LastDataTrue],
]

should_succeed_cases = [
    [Reqs.InitRequestPayload, Reqs.Payload1, Reqs.Payload2, Reqs.PayloadComplete, Reqs.InitData1, Reqs.Data1,
     Reqs.Data2, Reqs.DataComplete, Reqs.InitData2, Reqs.Data1, Reqs.Data2, Reqs.Data2, Reqs.Data2,
     Reqs.DataComplete, Reqs.LastDataTrue],
    [Reqs.InitRequestPayload, Reqs.Payload1, Reqs.PayloadComplete, Reqs.InitData1, Reqs.Data1, Reqs.DataComplete,
     Reqs.LastDataTrue],
    [Reqs.InitRequestPayload, Reqs.PayloadComplete, Reqs.InitData1, Reqs.Data1, Reqs.DataComplete, Reqs.LastDataTrue],
]


def get_cases(list_requests):
    for r in list_requests:
        yield iter(r)


@pytest.mark.parametrize("requests", get_cases(should_throw_cases))
def test_taskhandler_create_should_throw(requests: Iterator[ProcessRequest]):
    with pytest.raises(ValueError):
        TaskHandler.create(requests, DummyAgent(DummyChannel()))


@pytest.mark.parametrize("requests", get_cases(should_succeed_cases))
def test_taskhandler_create_should_succeed(requests: Iterator[ProcessRequest]):
    agent = DummyAgent(DummyChannel())
    task_handler = TaskHandler.create(requests, agent)
    assert task_handler.token is not None and len(task_handler.token) > 0
    assert len(task_handler.payload) > 0
    assert task_handler.session_id is not None and len(task_handler.session_id) > 0
    assert task_handler.task_id is not None and len(task_handler.task_id) > 0


def test_taskhandler_data_are_correct():
    agent = DummyAgent(DummyChannel())
    task_handler = TaskHandler.create(iter(should_succeed_cases[0]), agent)
    assert len(task_handler.payload) > 0
    assert task_handler.payload.decode('utf-8') == "testPayload1Payload2"
    assert len(task_handler.data_dependencies) == 2
    assert task_handler.data_dependencies["DataKey1"].decode('utf-8') == "Data1Data2"
    assert task_handler.data_dependencies["DataKey2"].decode('utf-8') == "Data1Data2Data2Data2"
    assert task_handler.task_id == "TaskId"
    assert task_handler.session_id == "SessionId"
    assert task_handler.token == "Token"

    task_handler.send_result("test", "TestData".encode("utf-8"))

    results = agent.send_result_task_message
    assert len(results) == 4
    assert results[0].WhichOneof("type") == "init"
    assert results[0].init.key == "test"
    assert results[1].WhichOneof("type") == "data"
    assert results[1].data.data == "TestData".encode("utf-8")
    assert results[2].WhichOneof("type") == "data"
    assert results[2].data.data_complete
    assert results[3].WhichOneof("type") == "init"
    assert results[3].init.last_result

    task_handler.create_tasks([TaskDefinition("Payload".encode("utf-8"), ["EOK"], ["DD"])])

    tasks = agent.create_task_messages
    assert len(tasks) == 5
    assert tasks[0].WhichOneof("type") == "init_request"
    assert tasks[1].WhichOneof("type") == "init_task"
    assert len(tasks[1].init_task.header.data_dependencies) == 1 \
           and tasks[1].init_task.header.data_dependencies[0] == "DD"
    assert len(tasks[1].init_task.header.expected_output_keys) == 1 \
           and tasks[1].init_task.header.expected_output_keys[0] == "EOK"
    assert tasks[2].WhichOneof("type") == "task_payload"
    assert tasks[2].task_payload.data == "Payload".encode("utf-8")
    assert tasks[3].WhichOneof("type") == "task_payload"
    assert tasks[3].task_payload.data_complete
    assert tasks[4].WhichOneof("type") == "init_task"
    assert tasks[4].init_task.last_task


