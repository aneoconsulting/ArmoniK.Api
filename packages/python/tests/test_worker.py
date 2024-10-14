import datetime
import logging
import os

from armonik.common.channel import create_channel

from .conftest import data_folder, grpc_endpoint, ca_cert, client_cert, client_key
from armonik.worker import ArmoniKWorker, TaskHandler, ClefLogger
from armonik.common import Output, TaskOptions
from armonik.protogen.common.objects_pb2 import Empty, Configuration
from armonik.protogen.common.worker_common_pb2 import ProcessRequest


def do_nothing(_: TaskHandler) -> Output:
    return Output()


def throw_error(_: TaskHandler) -> Output:
    raise ValueError("TestError")


def return_error(_: TaskHandler) -> Output:
    return Output("TestError")


def return_and_send(th: TaskHandler) -> Output:
    th.send_results({th.expected_results[0]: b"result"})
    return Output()


class TestWorker:
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

    def test_do_nothing(self):
        with create_channel(
            grpc_endpoint,
            certificate_authority=ca_cert,
            client_certificate=client_cert,
            client_key=client_key,
        ) as agent_channel:
            worker = ArmoniKWorker(
                agent_channel,
                do_nothing,
                logger=ClefLogger("TestLogger", level=logging.CRITICAL),
            )
            reply = worker.Process(self.request, None)
            assert Output(
                reply.output.error.details if reply.output.WhichOneof("type") == "error" else None
            ).success
            worker.HealthCheck(Empty(), None)

    def test_should_return_none(self):
        with create_channel(
            grpc_endpoint,
            certificate_authority=ca_cert,
            client_certificate=client_cert,
            client_key=client_key,
        ) as agent_channel:
            worker = ArmoniKWorker(
                agent_channel,
                throw_error,
                logger=ClefLogger("TestLogger", level=logging.CRITICAL),
            )
            reply = worker.Process(self.request, None)
            assert reply is None

    def test_should_error(self):
        with create_channel(
            grpc_endpoint,
            certificate_authority=ca_cert,
            client_certificate=client_cert,
            client_key=client_key,
        ) as agent_channel:
            worker = ArmoniKWorker(
                agent_channel,
                return_error,
                logger=ClefLogger("TestLogger", level=logging.CRITICAL),
            )
            reply = worker.Process(self.request, None)
            output = Output(
                reply.output.error.details if reply.output.WhichOneof("type") == "error" else None
            )
            assert not output.success
            assert output.error == "TestError"

    def test_should_write_result(self):
        with create_channel(
            grpc_endpoint,
            certificate_authority=ca_cert,
            client_certificate=client_cert,
            client_key=client_key,
        ) as agent_channel:
            worker = ArmoniKWorker(
                agent_channel,
                return_and_send,
                logger=ClefLogger("TestLogger", level=logging.DEBUG),
            )
            reply = worker.Process(self.request, None)
            assert reply is not None
            output = Output(
                reply.output.error.details if reply.output.WhichOneof("type") == "error" else None
            )
            assert output.success
            assert os.path.exists(os.path.join(data_folder, self.request.expected_output_keys[0]))
            with open(os.path.join(data_folder, self.request.expected_output_keys[0]), "rb") as f:
                value = f.read()
                assert len(value) > 0
