#!/usr/bin/env python3
import os

import pytest
from typing import Iterator

from armonik.common import TaskDefinition

from .common import DummyChannel
from armonik.worker import TaskHandler
from armonik.protogen.worker.agent_service_pb2_grpc import AgentStub
from armonik.protogen.common.agent_common_pb2 import CreateTaskRequest, CreateTaskReply, NotifyResultDataRequest, NotifyResultDataResponse
from armonik.protogen.common.worker_common_pb2 import ProcessRequest
from armonik.protogen.common.objects_pb2 import Configuration
import logging

logging.basicConfig()
logging.getLogger().setLevel(logging.INFO)

data_folder = os.getcwd()


@pytest.fixture(autouse=True, scope="session")
def setup_teardown():
    with open(os.path.join(data_folder, "payloadid"), "wb") as f:
        f.write("payload".encode())
    with open(os.path.join(data_folder, "ddid"), "wb") as f:
        f.write("dd".encode())
    yield
    os.remove(os.path.join(data_folder, "payloadid"))
    os.remove(os.path.join(data_folder, "ddid"))


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

    def NotifyResultData(self, request: NotifyResultDataRequest) -> NotifyResultDataResponse:
        self.send_result_task_message.append(request)
        return NotifyResultDataResponse(result_ids=[i.result_id for i in request.ids])


should_succeed_case = ProcessRequest(communication_token="token", session_id="sessionid", task_id="taskid", expected_output_keys=["resultid"], payload_id="payloadid", data_dependencies=["ddid"], data_folder=data_folder, configuration=Configuration(data_chunk_max_size=8000))


@pytest.mark.parametrize("requests", [should_succeed_case])
def test_taskhandler_create_should_succeed(requests: ProcessRequest):
    agent = DummyAgent(DummyChannel())
    task_handler = TaskHandler(requests, agent)
    assert task_handler.token is not None and len(task_handler.token) > 0
    assert len(task_handler.payload) > 0
    assert task_handler.session_id is not None and len(task_handler.session_id) > 0
    assert task_handler.task_id is not None and len(task_handler.task_id) > 0


def test_taskhandler_data_are_correct():
    agent = DummyAgent(DummyChannel())
    task_handler = TaskHandler(should_succeed_case, agent)
    assert len(task_handler.payload) > 0

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


