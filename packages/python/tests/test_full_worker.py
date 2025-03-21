import multiprocessing
from datetime import timedelta

import pytest
from armonik.common import TaskOptions, Output
from armonik.protogen.common.objects_pb2 import Configuration
from armonik.protogen.common.worker_common_pb2 import ProcessRequest
from armonik.worker import armonik_worker, TaskHandler

from .conftest import (
    call_me_with_healthcheck,
    call_me_with_process,
    data_folder,
)

payload_id = "payload_id"
data_dependencies = ["dd_0_id", "dd_1_id"]
expected_output_keys = ["eok_0", "eok_1"]
token = "comm_token"
session_id = "session_id"
task_id = "task_id"

data_chunk_max_size = 84000
results = {
    payload_id: b"payload",
    data_dependencies[0]: b"dd_0",
    data_dependencies[1]: b"dd_1",
}
task_options = TaskOptions(
    max_duration=timedelta(minutes=5),
    priority=3,
    max_retries=10,
    partition_id="partition_id",
    application_name="application_name",
    application_version="application_version",
    application_namespace="application_namespace",
    application_service="application_service",
    options={"option1_key": "option1", "option2_key": "option2"},
)


@armonik_worker()
def worker(task_handler: TaskHandler):
    assert task_handler.payload == results[payload_id]
    assert len(task_handler.data_dependencies) == len(data_dependencies)
    assert task_handler.data_dependencies[data_dependencies[0]] == results[data_dependencies[0]]
    assert (
        len(
            set(task_handler.data_dependencies.values()).symmetric_difference(
                (results[data_dependencies[0]], results[data_dependencies[1]])
            )
        )
        == 0
    )
    assert task_handler.task_options.max_duration == task_options.max_duration
    assert task_handler.task_options.priority == task_options.priority
    assert task_handler.task_options.max_retries == task_options.max_retries
    assert task_handler.task_options.partition_id == task_options.partition_id
    assert task_handler.task_options.application_name == task_options.application_name
    assert task_handler.task_options.application_version == task_options.application_version
    assert task_handler.task_options.application_namespace == task_options.application_namespace
    assert task_handler.task_options.application_service == task_options.application_service
    assert (
        len(
            set(task_handler.task_options.options.keys()).symmetric_difference(
                task_options.options.keys()
            )
        )
        == 0
    )
    assert (
        len(
            set(task_handler.task_options.options.values()).symmetric_difference(
                task_options.options.values()
            )
        )
        == 0
    )
    assert task_handler.configuration.data_chunk_max_size == data_chunk_max_size
    assert (
        len(set(task_handler.expected_results).symmetric_difference(set(expected_output_keys))) == 0
    )
    assert task_handler.token == token
    assert task_handler.session_id == session_id
    assert task_handler.task_id == task_id
    return Output()


@pytest.fixture
def worker_server():
    server = multiprocessing.Process(target=worker.run)
    server.start()
    yield
    server.terminate()
    server.join()


def test_worker_healthcheck(worker_server):
    _ = worker_server
    call_me_with_healthcheck()


def test_worker_process(worker_server):
    _ = worker_server
    call_me_with_process(
        request=ProcessRequest(
            communication_token=token,
            session_id=session_id,
            task_id=task_id,
            task_options=task_options.to_message(),
            expected_output_keys=expected_output_keys,
            payload_id=payload_id,
            data_dependencies=data_dependencies,
            data_folder=data_folder,
            configuration=Configuration(data_chunk_max_size=data_chunk_max_size),
        ),
        results=results,
    )
