#!/usr/bin/env python3
import logging

from armonik.worker import ArmoniKWorker, TaskHandler, ClefLogger
from armonik.common import Output
from .taskhandler_test import should_succeed_cases
from armonik.protogen.common.objects_pb2 import Empty
import grpc


def do_nothing(_: TaskHandler) -> Output:
    return Output()


def throw_error(_: TaskHandler) -> Output:
    raise ValueError("TestError")


def return_error(_: TaskHandler) -> Output:
    return Output("TestError")


def test_do_nothing_worker():
    with grpc.insecure_channel("unix:///tmp/agent.sock") as agent_channel:
        worker = ArmoniKWorker(agent_channel, do_nothing, logger=ClefLogger("TestLogger", level=logging.CRITICAL))
        reply = worker.Process(iter(should_succeed_cases[0]), None)
        assert Output(reply.output.error.details if reply.output.WhichOneof("type") == "error" else None).success
        worker.HealthCheck(Empty(), None)


def test_worker_should_return_none():
    with grpc.insecure_channel("unix:///tmp/agent.sock") as agent_channel:
        worker = ArmoniKWorker(agent_channel, throw_error, logger=ClefLogger("TestLogger", level=logging.CRITICAL))
        reply = worker.Process(iter(should_succeed_cases[0]), None)
        assert reply is None


def test_worker_should_error():
    with grpc.insecure_channel("unix:///tmp/agent.sock") as agent_channel:
        worker = ArmoniKWorker(agent_channel, return_error, logger=ClefLogger("TestLogger", level=logging.CRITICAL))
        reply = worker.Process(iter(should_succeed_cases[0]), None)
        output = Output(reply.output.error.details if reply.output.WhichOneof("type") == "error" else None)
        assert not output.success
        assert output.error == "TestError"

