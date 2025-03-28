from __future__ import annotations

import logging
import os
from os import PathLike
import traceback
from concurrent import futures
from contextlib import nullcontext
from logging import Logger
from typing import Callable, Union, Optional, Tuple, Iterable

import grpc
from grpc import Channel
from grpc._server import _Server

from ..common.channel import create_channel, get_scheme_and_endpoint_from_uri
from .seqlogger import ClefLogger
from ..common import Output, HealthCheckStatus
from ..protogen.common.objects_pb2 import Empty
from ..protogen.common.worker_common_pb2 import (
    ProcessReply,
    ProcessRequest,
    HealthCheckReply,
)
from ..protogen.worker.agent_service_pb2_grpc import AgentStub
from ..protogen.worker.worker_service_pb2_grpc import (
    WorkerServicer,
    add_WorkerServicer_to_server,
)
from .taskhandler import TaskHandler


class ArmoniKWorker(WorkerServicer):
    def __init__(
        self,
        agent_channel: Channel,
        processing_function: Callable[[TaskHandler], Output],
        health_check: Callable[
            [], HealthCheckReply.ServingStatus
        ] = lambda: HealthCheckStatus.SERVING,
        logger=ClefLogger.getLogger("ArmoniKWorker"),
    ):
        """Creates a worker for ArmoniK

        Args:
            agent_channel: gRPC channel for the agent
            processing_function: Function that will be called when a
                task needs processing
            health_check: Function that will be called to check the
                health of the worker. Defaults to a simple "Serving"
                reply
            logger: Logger used for the worker, defaults to a logger
                ArmoniKWorker compatible with Seq
        """
        self.health_check = health_check
        self.processing_function = processing_function
        self._client = AgentStub(agent_channel)
        self._logger = logger
        self._server: Optional[_Server] = None

    def start(self, endpoint: str, wait=True):
        """Starts the worker

        Args:
            endpoint: endpoint from which to listen to requests
            wait: Wait until the server stops
        """
        self._server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
        add_WorkerServicer_to_server(self, self._server)
        self._server.add_insecure_port(endpoint)
        self._server.start()
        if wait:
            self._server.wait_for_termination()

    def stop(self):
        """
        Stop the server
        """
        if self._server is not None:
            self._server.stop(grace=None)
            self._server.wait_for_termination()
        self._server = None

    def Process(self, request: ProcessRequest, context) -> Union[ProcessReply, None]:
        try:
            self._logger.debug("Received task")
            task_handler = TaskHandler(request, self._client)
            return ProcessReply(output=self.processing_function(task_handler).to_message())
        except Exception as e:
            self._logger.exception(
                f"Failed task {''.join(traceback.format_exception(type(e), e, e.__traceback__))}",
                exc_info=e,
            )

    def HealthCheck(self, request: Empty, context) -> HealthCheckReply:
        return HealthCheckReply(status=self.health_check())


class ArmoniKWorkerWrapper:
    def __init__(
        self,
        *,
        processor: Callable[[TaskHandler], Output],
        logger: Optional[Logger] = None,
        worker_endpoint: Optional[str] = None,
        agent_endpoint: Optional[str] = None,
        channel_options: Optional[Iterable[Tuple[str, str]]] = None,
        agent_certificate_authority: Union[str, PathLike, bytes, None] = None,
        agent_client_certificate: Union[str, PathLike, bytes, None] = None,
        agent_client_key: Union[str, PathLike, bytes, None] = None,
    ):
        if logger is None:
            ClefLogger.setup_logging(logging.INFO)
            logger = ClefLogger.getLogger("ArmoniKWorker")
        if worker_endpoint is None:
            _, worker_endpoint = get_scheme_and_endpoint_from_uri(
                os.getenv("ComputePlane__WorkerChannel__Address", "/cache/armonik_worker.sock"),
                os.getenv("ComputePlane__WorkerChannel__SocketType", "unixdomainsocket"),
            )
        if agent_endpoint is None:
            agent_scheme, agent_endpoint = get_scheme_and_endpoint_from_uri(
                os.getenv("ComputePlane__AgentChannel__Address", "/cache/armonik_agent.sock"),
                os.getenv("ComputePlane__AgentChannel__SocketType", "unixdomainsocket"),
                keep_scheme=True,
            )
        else:
            agent_scheme, _ = get_scheme_and_endpoint_from_uri(agent_endpoint)
        if channel_options is None and "unix" in agent_scheme:
            channel_options = (("grpc.default_authority", "localhost"),)
        self.logger = logger
        self.worker_endpoint = worker_endpoint
        self.agent_endpoint = agent_endpoint
        self.channel_options = channel_options
        self.processor = processor
        self._worker: Optional[ArmoniKWorker] = None
        self._agent_channel: Optional[grpc.Channel] = None
        self._agent_certificate_authority = agent_certificate_authority
        self._agent_client_certificate = agent_client_certificate
        self._agent_client_key = agent_client_key

    def __call__(self, *args, **kwargs):
        return self.processor(*args, **kwargs)

    def run(
        self,
        agent_channel: Optional[Channel] = None,
        logger: Optional[Logger] = None,
        worker_endpoint: Optional[str] = None,
        wait=True,
    ):
        """
        Run the server
        Args:
            agent_channel: Agent channel
            logger: Logger
            worker_endpoint: Worker endpoint
            wait: Wait until the server stops
        Returns:
            None
        """
        logger = self.logger if logger is None else logger
        worker_endpoint = self.worker_endpoint if worker_endpoint is None else worker_endpoint
        # Start worker
        logger.info("Worker Started")
        if agent_channel is None:
            self._agent_channel = create_channel(
                self.agent_endpoint,
                options=self.channel_options,
                certificate_authority=self._agent_certificate_authority,
                client_certificate=self._agent_client_certificate,
                client_key=self._agent_client_key,
            )
            agent_channel = self._agent_channel
        else:
            agent_channel = nullcontext(agent_channel)
        if wait:
            with agent_channel as channel:
                self._worker = ArmoniKWorker(channel, self.processor, logger=logger)
                logger.info("Worker Connected")
                self._worker.start(worker_endpoint, wait=wait)
        else:
            self._worker = ArmoniKWorker(agent_channel.__enter__(), self.processor, logger=logger)
            logger.info("Worker Connected")
            self._worker.start(worker_endpoint, wait=wait)

    def stop(self):
        """Stop the server"""
        self._worker.stop()
        if self._agent_channel is not None:
            self._agent_channel.__exit__(None, None, None)


def armonik_worker(
    *,
    autorun: bool = False,
    logger: Optional[Logger] = None,
    worker_endpoint: Optional[str] = None,
    agent_endpoint: Optional[str] = None,
    channel_options: Optional[Iterable[Tuple[str, str]]] = None,
    agent_certificate_authority: Union[str, PathLike, bytes, None] = None,
    agent_client_certificate: Union[str, PathLike, bytes, None] = None,
    agent_client_key: Union[str, PathLike, bytes, None] = None,
):
    """
    Transforms the function into an ArmoniK Worker
    Args:
        autorun: if True, runs the processor instead of returning the function
        logger: Logger to use, if None will use the default ClefLogger
        worker_endpoint: Worker endpoint, if None will use the default from ComputePlane__WorkerChannel__SocketType and ComputePlane__WorkerChannel__Address
        agent_endpoint: Agent endpoint, if None will use the default from ComputePlane__AgentChannel__SocketType and ComputePlane__AgentChannel__Address
        channel_options: Options for the gRPC channel
        agent_certificate_authority: Certificate Authority for agent
        agent_client_certificate: Client Certificate for agent
        agent_client_key: Client Key for agent

    Returns:
        Worker function

    Example:
        >>> @armonik_worker()
        >>> def processor(task_handler: TaskHandler) -> Output:
        >>>    ...
        >>>    return Output()
    """

    def decorator(
        processor: Callable[[TaskHandler], Output],
    ) -> ArmoniKWorkerWrapper:
        worker = ArmoniKWorkerWrapper(
            processor=processor,
            logger=logger,
            worker_endpoint=worker_endpoint,
            agent_endpoint=agent_endpoint,
            channel_options=channel_options,
            agent_certificate_authority=agent_certificate_authority,
            agent_client_certificate=agent_client_certificate,
            agent_client_key=agent_client_key,
        )
        if autorun:
            worker.run()
        return worker

    return decorator
