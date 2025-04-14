import os
from datetime import timedelta

import pytest
from armonik.common import TaskOptions, Output
from armonik.protogen.common.objects_pb2 import Configuration
from armonik.protogen.common.worker_common_pb2 import ProcessRequest
from armonik.worker import armonik_worker, TaskHandler

from .conftest import ca_cert, client_cert, client_key

from .conftest import (
    call_me_with_healthcheck,
    call_me_with_process,
    data_folder,
    rpc_called,
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


@armonik_worker(
    agent_client_certificate=client_cert,
    agent_client_key=client_key,
    agent_certificate_authority=ca_cert,
)
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
    task_handler.send_results({k: k.encode() for k in expected_output_keys})
    return Output()


@pytest.fixture
def worker_server(clean_up):
    worker.run(wait=False)
    yield
    worker.stop()


@pytest.fixture
def clean_up_data_folder():
    yield
    for k in data_dependencies + expected_output_keys + [payload_id]:
        p = os.path.join(data_folder, k)
        if os.path.exists(p):
            os.remove(p)


@pytest.mark.worker
class TestFullWorker:
    def test_worker_healthcheck(self, worker_server):
        _ = worker_server
        reply = call_me_with_healthcheck()
        assert isinstance(reply, dict), str(reply)
        assert reply["status"] == "SERVING", str(reply)

    def test_worker_process(self, worker_server):
        _ = worker_server
        reply = call_me_with_process(
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
        assert isinstance(reply, dict), str(reply)
        assert reply.get("output", {}).get("ok") is not None, str(reply)
        assert reply.get("output", {}).get("error") is None, str(reply)
        assert rpc_called("Agent", "NotifyResultData")
