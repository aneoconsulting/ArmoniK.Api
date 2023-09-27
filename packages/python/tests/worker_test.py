#!/usr/bin/env python3
import logging
import os
import pytest
from armonik.worker import ArmoniKWorker, TaskHandler, ClefLogger
from armonik.common import Output
from .taskhandler_test import should_succeed_case, data_folder, DummyAgent
from .common import DummyChannel
from armonik.protogen.common.objects_pb2 import Empty
import grpc


def do_nothing(_: TaskHandler) -> Output:
    return Output()


def throw_error(_: TaskHandler) -> Output:
    raise ValueError("TestError")


def return_error(_: TaskHandler) -> Output:
    return Output("TestError")


def return_and_send(th: TaskHandler) -> Output:
    th.send_result(th.expected_results[0], b"result")
    return Output()


@pytest.fixture(autouse=True, scope="function")
def remove_result():
    yield
    if os.path.exists(os.path.join(data_folder, "resultid")):
        os.remove(os.path.join(data_folder, "resultid"))


def test_do_nothing_worker():
    with grpc.insecure_channel("unix:///tmp/agent.sock") as agent_channel:
        worker = ArmoniKWorker(agent_channel, do_nothing, logger=ClefLogger("TestLogger", level=logging.CRITICAL))
        reply = worker.Process(should_succeed_case, None)
        assert Output(reply.output.error.details if reply.output.WhichOneof("type") == "error" else None).success
        worker.HealthCheck(Empty(), None)


def test_worker_should_return_none():
    with grpc.insecure_channel("unix:///tmp/agent.sock") as agent_channel:
        worker = ArmoniKWorker(agent_channel, throw_error, logger=ClefLogger("TestLogger", level=logging.CRITICAL))
        reply = worker.Process(should_succeed_case, None)
        assert reply is None


def test_worker_should_error():
    with grpc.insecure_channel("unix:///tmp/agent.sock") as agent_channel:
        worker = ArmoniKWorker(agent_channel, return_error, logger=ClefLogger("TestLogger", level=logging.CRITICAL))
        reply = worker.Process(should_succeed_case, None)
        output = Output(reply.output.error.details if reply.output.WhichOneof("type") == "error" else None)
        assert not output.success
        assert output.error == "TestError"


def test_worker_should_write_result():
    with grpc.insecure_channel("unix:///tmp/agent.sock") as agent_channel:
        worker = ArmoniKWorker(agent_channel, return_and_send, logger=ClefLogger("TestLogger", level=logging.DEBUG))
        worker._client = DummyAgent(DummyChannel())
        reply = worker.Process(should_succeed_case, None)
        assert reply is not None
        output = Output(reply.output.error.details if reply.output.WhichOneof("type") == "error" else None)
        assert output.success
        assert os.path.exists(os.path.join(data_folder, should_succeed_case.expected_output_keys[0]))
        with open(os.path.join(data_folder, should_succeed_case.expected_output_keys[0]), "rb") as f:
            value = f.read()
            assert len(value) > 0

