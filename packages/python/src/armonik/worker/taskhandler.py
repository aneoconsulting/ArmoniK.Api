from __future__ import annotations
import os
from typing import Optional, Dict, List, Tuple, Union, cast

from ..common import TaskOptions, TaskDefinition, Task
from ..protogen.common.agent_common_pb2 import CreateTaskRequest, CreateResultsMetaDataRequest, CreateResultsMetaDataResponse, NotifyResultDataRequest
from ..protogen.common.objects_pb2 import TaskRequest, DataChunk, InitTaskRequest, TaskRequestHeader, Configuration
from ..protogen.worker.agent_service_pb2_grpc import AgentStub
from ..protogen.common.worker_common_pb2 import ProcessRequest


class TaskHandler:
    def __init__(self, request: ProcessRequest, agent_client: AgentStub):
        self._client: AgentStub = agent_client
        self.session_id: str = request.session_id
        self.task_id: str = request.task_id
        self.task_options: TaskOptions = TaskOptions.from_message(request.task_options)
        self.token: str = request.communication_token
        self.expected_results: List[str] = list(request.expected_output_keys)
        self.configuration: Configuration = request.configuration
        self.payload_id: str = request.payload_id
        self.data_folder: str = request.data_folder

        # TODO: Lazy load
        with open(os.path.join(self.data_folder, self.payload_id), "rb") as f:
            self.payload = f.read()

        # TODO: Lazy load
        self.data_dependencies: Dict[str, bytes] = {}
        for dd in request.data_dependencies:
            with open(os.path.join(self.data_folder, dd), "rb") as f:
                self.data_dependencies[dd] = f.read()

    def create_tasks(self, tasks: List[TaskDefinition], task_options: Optional[TaskOptions] = None) -> Tuple[List[Task], List[str]]:
        """Create new tasks for ArmoniK

        Args:
            tasks: List of task definitions
            task_options: Task Options used for this batch of tasks

        Returns:
            Tuple containing the list of successfully sent tasks, and
            the list of submission errors if any
        """
        task_requests = []

        for t in tasks:
            task_request = TaskRequest()
            task_request.expected_output_keys.extend(t.expected_output_ids)
            task_request.data_dependencies.extend(t.data_dependencies)
            task_request.payload = t.payload
            task_requests.append(task_request)
        assert self.configuration is not None
        create_tasks_reply = self._client.CreateTask(_to_request_stream(task_requests, self.token, task_options.to_message() if task_options is not None else None, self.configuration.data_chunk_max_size))
        ret = create_tasks_reply.WhichOneof("Response")
        if ret is None or ret == "error":
            raise Exception(f'Issue with server when submitting tasks : {create_tasks_reply.error}')
        elif ret == "creation_status_list":
            tasks_created = []
            tasks_creation_failed = []
            for creation_status in create_tasks_reply.creation_status_list.creation_statuses:
                if creation_status.WhichOneof("Status") == "task_info":
                    tasks_created.append(Task(id=creation_status.task_info.task_id, session_id=self.session_id, expected_output_ids=[k for k in creation_status.task_info.expected_output_keys], data_dependencies=[k for k in creation_status.task_info.data_dependencies]))
                else:
                    tasks_creation_failed.append(creation_status.error)
        else:
            raise Exception("Unknown value")
        return tasks_created, tasks_creation_failed

    def send_result(self, key: str, data: Union[bytes, bytearray]) -> None:
        """ Send task result

        Args:
            key: Result key
            data: Result data
        """
        with open(os.path.join(self.data_folder, key), "wb") as f:
            f.write(data)

        self._client.NotifyResultData(NotifyResultDataRequest(ids=[NotifyResultDataRequest.ResultIdentifier(session_id=self.session_id, result_id=key)], communication_token=self.token))

    def get_results_ids(self, names: List[str]) -> Dict[str, str]:
        return {r.name: r.result_id for r in cast(CreateResultsMetaDataResponse, self._client.CreateResultsMetaData(CreateResultsMetaDataRequest(results=[CreateResultsMetaDataRequest.ResultCreate(name=n) for n in names], session_id=self.session_id, communication_token=self.token))).results}


def _to_request_stream_internal(request, communication_token, is_last, chunk_max_size):
    req = CreateTaskRequest(
        init_task=InitTaskRequest(
            header=TaskRequestHeader(
                data_dependencies=request.data_dependencies,
                expected_output_keys=request.expected_output_keys
            )
        ),
        communication_token=communication_token
    )
    yield req
    start = 0
    payload_length = len(request.payload)
    if payload_length == 0:
        req = CreateTaskRequest(
            task_payload=DataChunk(data=b''),
            communication_token=communication_token
        )
        yield req
    while start < payload_length:
        chunk_size = min(chunk_max_size, payload_length - start)
        req = CreateTaskRequest(
            task_payload=DataChunk(data=request.payload[start:start + chunk_size]),
            communication_token=communication_token
        )
        yield req
        start += chunk_size
    req = CreateTaskRequest(
        task_payload=DataChunk(data_complete=True),
        communication_token=communication_token
    )
    yield req

    if is_last:
        req = CreateTaskRequest(
            init_task=InitTaskRequest(last_task=True),
            communication_token=communication_token
        )
        yield req


def _to_request_stream(requests, communication_token, t_options, chunk_max_size):
    if t_options is None:
        req = CreateTaskRequest(
            init_request=CreateTaskRequest.InitRequest(),
            communication_token=communication_token)
    else:
        req = CreateTaskRequest(
            init_request=CreateTaskRequest.InitRequest(task_options=t_options),
            communication_token=communication_token)
    yield req
    if len(requests) == 0:
        return
    for r in requests[:-1]:
        for req in _to_request_stream_internal(r, communication_token, False, chunk_max_size):
            yield req
    for req in _to_request_stream_internal(requests[-1], communication_token, True, chunk_max_size):
        yield req
