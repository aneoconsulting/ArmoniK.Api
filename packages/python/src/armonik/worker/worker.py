import logging
import traceback
from concurrent import futures
from typing import Callable

import grpc
from grpc import Channel

from .seqlogger import get_worker_logger
from ..common import Output, HealthCheckStatus
from ..protogen.common.worker_common_pb2 import ProcessReply, HealthCheckReply
from ..protogen.worker.agent_service_pb2_grpc import AgentStub
from ..protogen.worker.worker_service_pb2_grpc import WorkerServicer, add_WorkerServicer_to_server
from .taskhandler import TaskHandler


class ArmoniKWorker(WorkerServicer):
    def __init__(self, agent_channel: Channel, processing_function: Callable[[TaskHandler], Output], health_check: Callable[[], HealthCheckStatus] = lambda: HealthCheckStatus.SERVING, logger=get_worker_logger("ArmoniKWorker", logging.INFO)):
        self.health_check = health_check
        self.processing_function = processing_function
        self._client = AgentStub(agent_channel)
        self._logger = logger

    def start(self, endpoint: str):
        server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
        add_WorkerServicer_to_server(self, server)
        server.add_insecure_port(endpoint)
        server.start()
        server.wait_for_termination()

    def Process(self, request_iterator, context) -> ProcessReply:
        try:
            task_handler = TaskHandler.create(request_iterator, self._client)
            return ProcessReply(output=self.processing_function(task_handler).to_message())
        except Exception as e:
            self._logger.exception(f"Failed task {''.join(traceback.format_exception(e))}", exc_info=e)

    def HealthCheck(self, request, context) -> HealthCheckReply:
        return HealthCheckReply(status=self.health_check().value)
