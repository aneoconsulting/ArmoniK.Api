#!/usr/bin/env python3
import pytest
from typing import Iterator, Any
from ..worker import TaskHandler
from ..protogen.worker.agent_service_pb2_grpc import AgentServicer, AgentStub
from ..protogen.common.agent_common_pb2 import CreateTaskRequest, CreateTaskReply, Result, ResultReply
from ..protogen.common.worker_common_pb2 import ProcessRequest, ProcessReply
from ..protogen.common.objects_pb2 import Configuration, DataChunk, TaskOptions


class DummyChannel:
    def stream_unary(self, *args, **kwargs):
        pass

    def unary_stream(self, *args, **kwargs):
        pass

    def unary_unary(self, *args, **kwargs):
        pass

    def stream_stream(self, *args, **kwargs):
        pass


class DummyAgent(AgentStub):

    def CreateTask(self, request_iterator: Iterator[CreateTaskRequest]) -> CreateTaskReply:
        self.create_task_messages = [r for r in request_iterator]
        return CreateTaskReply()

    def SendResult(self, request_iterator: Iterator[Result]) -> ResultReply:
        self.send_result_task_message = [r for r in request_iterator]
        return ResultReply()

    def __init__(self, channel) -> None:
        super().__init__(channel)
        self.create_task_messages = []
        self.send_result_task_message = []


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


should_throw_cases = [
    (iter([])),
    (iter([Reqs.InitData1])),
    (iter([Reqs.InitData2])),
    (iter([Reqs.LastDataTrue])),
    (iter([Reqs.LastDataFalse])),
    (iter([Reqs.InitRequestPayload])),
]


@pytest.mark.parametrize("requests", should_throw_cases)
def test_TaskHandlerCreateShouldThrow(requests: Iterator[ProcessRequest]):
    with pytest.raises(ValueError):
        TaskHandler.create(requests, DummyAgent(DummyChannel()))
