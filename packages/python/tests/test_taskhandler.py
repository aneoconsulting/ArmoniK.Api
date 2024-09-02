import datetime
import logging

from .conftest import all_rpc_called, rpc_called, get_client, data_folder
from armonik.common import TaskDefinition, TaskOptions
from armonik.worker import TaskHandler
from armonik.protogen.common.worker_common_pb2 import ProcessRequest
from armonik.protogen.common.objects_pb2 import Configuration


logging.basicConfig()
logging.getLogger().setLevel(logging.INFO)


class TestTaskHandler:
    request = ProcessRequest(
        communication_token="token",
        session_id="session-id",
        task_id="task-id",
        expected_output_keys=["result-id"],
        payload_id="payload-id",
        data_dependencies=["dd-id"],
        data_folder=data_folder,
        configuration=Configuration(data_chunk_max_size=8000),
        task_options=TaskOptions(
            max_duration=datetime.timedelta(seconds=1), priority=1, max_retries=1
        ).to_message(),
    )

    def test_taskhandler_init(self):
        task_handler = TaskHandler(self.request, get_client("Agent"))

        assert task_handler.session_id == "session-id"
        assert task_handler.task_id == "task-id"
        assert task_handler.task_options == TaskOptions(
            max_duration=datetime.timedelta(seconds=1),
            priority=1,
            max_retries=1,
            partition_id="",
            application_name="",
            application_version="",
            application_namespace="",
            application_service="",
            engine_type="",
            options={},
        )
        assert task_handler.token == "token"
        assert task_handler.expected_results == ["result-id"]
        assert task_handler.configuration == Configuration(data_chunk_max_size=8000)
        assert task_handler.payload_id == "payload-id"
        assert task_handler.data_folder == data_folder
        assert task_handler.payload == "payload".encode()
        assert dict(task_handler.data_dependencies) == {"dd-id": "dd".encode()}

    def test_submit_tasks(self):
        task_handler = TaskHandler(self.request, get_client("Agent"))
        tasks = task_handler.submit_tasks(
            [
                TaskDefinition(
                    payload_id="payload-id",
                    expected_output_ids=["result-id"],
                    data_dependencies=[],
                )
            ]
        )

        assert rpc_called("Agent", "SubmitTasks")
        assert tasks is not None

    def test_send_results(self):
        task_handler = TaskHandler(self.request, get_client("Agent"))
        task_handler.send_results({"result-id": b"result data"})
        assert rpc_called("Agent", "NotifyResultData")

    def test_create_result_metadata(self):
        task_handler = TaskHandler(self.request, get_client("Agent"))
        results = task_handler.create_results_metadata(["result-name"])

        assert rpc_called("Agent", "CreateResultsMetaData")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert results == {}

    def test_create_results(self):
        task_handler = TaskHandler(self.request, get_client("Agent"))
        results = task_handler.create_results({"result-name": b"test data"})

        assert rpc_called("Agent", "CreateResults")
        assert results == {}

    def test_lazy_load(self):
        handler = TaskHandler(self.request, None)
        assert handler.data_dependencies._data["dd-id"] is None
        assert len(handler.data_dependencies._data) == 1
        for _ in handler.data_dependencies.keys():
            pass
        assert handler.data_dependencies._data["dd-id"] is None
        for _ in handler.data_dependencies:
            pass
        assert handler.data_dependencies._data["dd-id"] == b"dd"

    def test_service_fully_implemented(self):
        assert all_rpc_called(
            "Agent", missings=["CreateTask", "GetCommonData", "GetDirectData", "GetResourceData"]
        )
