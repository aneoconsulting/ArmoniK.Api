import uuid
from typing import Optional, Dict, List, Tuple, Union, cast

from ..common import TaskOptions, TaskDefinition, Task
from ..protogen.common.agent_common_pb2 import Result, CreateTaskRequest, CreateResultsMetaDataRequest, CreateResultsMetaDataResponse
from ..protogen.common.objects_pb2 import TaskRequest, InitKeyedDataStream, DataChunk, InitTaskRequest, TaskRequestHeader, Configuration
from ..protogen.worker.agent_service_pb2_grpc import AgentStub


class TaskHandler:
    def __init__(self, request_iterator, agent_client):
        self.request_iterator = request_iterator
        self._client: AgentStub = agent_client
        self.payload = bytearray()
        self.session_id: Optional[str] = None
        self.task_id: Optional[str] = None
        self.task_options: Optional[TaskOptions] = None
        self.token: Optional[str] = None
        self.expected_results: List[str] = []
        self.data_dependencies: Dict[str, bytearray] = {}
        self.configuration: Optional[Configuration] = None

    @classmethod
    def create(cls, request_iterator, agent_client) -> "TaskHandler":
        output = cls(request_iterator, agent_client)
        output.init()
        return output

    def init(self):
        current = next(self.request_iterator, None)
        if current is None:
            raise ValueError("Request stream ended unexpectedly")

        if current.compute.WhichOneof("type") != "init_request":
            raise ValueError("Expected a Compute request type with InitRequest to start the stream.")

        init_request = current.compute.init_request
        self.session_id = init_request.session_id
        self.task_id = init_request.task_id
        self.task_options = TaskOptions.from_message(init_request.task_options)
        self.expected_results = list(init_request.expected_output_keys)
        self.configuration = init_request.configuration
        self.token = current.communication_token

        datachunk = init_request.payload
        self.payload.extend(datachunk.data)
        while not datachunk.data_complete:
            current = next(self.request_iterator, None)
            if current is None:
                raise ValueError("Request stream ended unexpectedly")
            if current.compute.WhichOneof("type") != "payload":
                raise ValueError("Expected a Compute request type with Payload to continue the stream.")
            datachunk = current.compute.payload
            self.payload.extend(datachunk.data)

        while True:
            current = next(self.request_iterator, None)
            if current is None:
                raise ValueError("Request stream ended unexpectedly")
            if current.compute.WhichOneof("type") != "init_data":
                raise ValueError("Expected a Compute request type with InitData to continue the stream.")
            init_data = current.compute.init_data
            if not (init_data.key is None or init_data.key == ""):
                chunk = bytearray()
                while True:
                    current = next(self.request_iterator, None)
                    if current is None:
                        raise ValueError("Request stream ended unexpectedly")
                    if current.compute.WhichOneof("type") != "data":
                        raise ValueError("Expected a Compute request type with Data to continue the stream.")
                    datachunk = current.compute.data
                    if datachunk.WhichOneof("type") == "data":
                        chunk.extend(datachunk.data)
                    elif datachunk.WhichOneof("type") is None or datachunk.WhichOneof("type") == "":
                        raise ValueError("Expected a Compute request type with Datachunk Payload to continue the stream.")
                    elif datachunk.WhichOneof("type") == "data_complete":
                        break
                self.data_dependencies[init_data.key] = chunk
            else:
                break

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
        def result_stream():
            res = Result(communication_token=self.token, init=InitKeyedDataStream(key=key))
            assert self.configuration is not None
            yield res
            start = 0
            data_len = len(data)
            while start < data_len:
                chunksize = min(self.configuration.data_chunk_max_size, data_len - start)
                res = Result(communication_token=self.token, data=DataChunk(data=data[start:start + chunksize]))
                yield res
                start += chunksize
            res = Result(communication_token=self.token, data=DataChunk(data_complete=True))
            yield res
            res = Result(communication_token=self.token, init=InitKeyedDataStream(last_result=True))
            yield res

        result_reply = self._client.SendResult(result_stream())
        if result_reply.WhichOneof("type") == "error":
            raise Exception(f"Cannot send result id={key}")
    
    def get_results_ids(self, names : List[str]) -> Dict[str, str]:
        return {r.name : r.result_id for r in cast(CreateResultsMetaDataResponse, self._client.CreateResultsMetaData(CreateResultsMetaDataRequest(results=[CreateResultsMetaDataRequest.ResultCreate(name = n) for n in names], session_id=self.session_id, communication_token=self.token))).results}



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
