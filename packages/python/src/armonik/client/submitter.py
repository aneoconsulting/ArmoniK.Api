import uuid
from typing import Optional, List, Tuple, Dict

from grpc import Channel

from ..common import get_task_filter, TaskOptions, TaskDefinition, Task, TaskStatus
from ..protogen.client.submitter_service_pb2_grpc import SubmitterStub
from ..protogen.common.objects_pb2 import Empty, TaskRequest, ResultRequest, DataChunk, InitTaskRequest, \
    TaskRequestHeader, Configuration
from ..protogen.common.submitter_common_pb2 import CreateSessionRequest, GetTaskStatusRequest, CreateLargeTaskRequest, \
    WaitRequest

"""
  rpc GetServiceConfiguration(Empty) returns (Configuration);

  rpc CreateSession(CreateSessionRequest) returns (CreateSessionReply);
  rpc CancelSession(Session) returns (Empty);

  rpc CreateSmallTasks(CreateSmallTaskRequest) returns (CreateTaskReply);
  rpc CreateLargeTasks(stream CreateLargeTaskRequest) returns (CreateTaskReply);

  rpc ListTasks(TaskFilter) returns (TaskIdList);
  rpc ListSessions(SessionFilter) returns (SessionIdList);

  rpc CountTasks(TaskFilter) returns (Count);
  rpc TryGetResultStream(ResultRequest) returns (stream ResultReply);
  rpc TryGetTaskOutput(TaskOutputRequest) returns (Output);
  rpc WaitForAvailability(ResultRequest) returns (AvailabilityReply) {
    option deprecated = true;
  }
  rpc WaitForCompletion(WaitRequest) returns (Count);
  rpc CancelTasks(TaskFilter) returns (Empty);
  rpc GetTaskStatus(GetTaskStatusRequest) returns (GetTaskStatusReply);
  rpc GetResultStatus(GetResultStatusRequest) returns (GetResultStatusReply) {
    option deprecated = true;
  }
"""


class ArmoniKSubmitter:
    def __init__(self, grpc_channel: Channel):
        self._client = SubmitterStub(grpc_channel)

    def get_service_configuration(self) -> Configuration:
        return self._client.GetServiceConfiguration(Empty())

    def create_session(self, default_task_options: TaskOptions, partition_ids: Optional[List[str]] = None) -> str:
        if partition_ids is None:
            partition_ids = []
        request = CreateSessionRequest(default_task_option=default_task_options)
        for partition in partition_ids:
            request.partition_ids.append(partition)
        return self._client.CreateSession(request).session_id

    def submit(self, session_id: str, tasks: List[TaskDefinition], task_options: Optional[TaskOptions] = None) -> Tuple[List[Task], List[str]]:
        task_requests = []

        for t in tasks:
            task_request = TaskRequest()
            task_request.expected_output_keys.extend(t.expected_outputs)
            if t.data_dependencies is not None:
                task_request.data_dependencies.extend(t.data_dependencies)
            task_request.payload = t.payload
            task_requests.append(task_request)

        configuration = self.get_service_configuration()
        create_tasks_reply = self._client.CreateLargeTasks(
            to_request_stream(task_requests, session_id, task_options, configuration.data_chunk_max_size))
        ret = create_tasks_reply.WhichOneof("Response")
        if ret is None or ret == "error":
            raise Exception(f'Issue with server when submitting tasks : {create_tasks_reply.error}')
        elif ret == "creation_status_list":
            tasks_created = []
            tasks_creation_failed = []
            for creation_status in create_tasks_reply.creation_status_list.creation_statuses:
                if creation_status.WhichOneof("Status") == "task_info":
                    tasks_created.append(Task(id=creation_status.task_info.task_id, session_id=session_id,
                                              expected_output_ids=[k for k in
                                                                   creation_status.task_info.expected_output_keys],
                                              data_dependencies=[k for k in
                                                                 creation_status.task_info.data_dependencies]))
                else:
                    tasks_creation_failed.append(creation_status.error)
        else:
            raise Exception("Unknown value")
        return tasks_created, tasks_creation_failed

    def list_tasks(self, session_ids: Optional[List[str]] = None, task_ids: Optional[List[str]] = None,
                   included_statuses: Optional[List[TaskStatus]] = None,
                   excluded_statuses: Optional[List[TaskStatus]] = None) -> List[str]:
        return [t for t in self._client.ListTasks(
            get_task_filter(session_ids, task_ids, included_statuses, excluded_statuses)).task_ids]

    def get_task_status(self, task_ids: List[str]) -> Dict[str, TaskStatus]:
        request = GetTaskStatusRequest()
        request.task_ids.extend(task_ids)
        reply = self._client.GetTaskStatus(request)
        return dict([(s.task_id, s.status) for s in reply.id_statuses])

    def wait_for_completion(self,
                            session_ids: Optional[List[str]] = None,
                            task_ids: Optional[List[str]] = None,
                            included_statuses: Optional[List[TaskStatus]] = None,
                            excluded_statuses: Optional[List[TaskStatus]] = None,
                            stop_on_first_task_error: bool = False,
                            stop_on_first_task_cancellation: bool = False) -> Dict[TaskStatus, int]:
        return dict([(sc.status, sc.count) for sc in self._client.WaitForCompletion(
            WaitRequest(filter=get_task_filter(session_ids, task_ids, included_statuses, excluded_statuses),
                        stop_on_first_task_error=stop_on_first_task_error,
                        stop_on_first_task_cancellation=stop_on_first_task_cancellation)).values])

    def get_result(self, session_id: str, result_id) -> bytes:
        result_request = ResultRequest(
            result_id=result_id,
            session=session_id
        )
        streaming_call = self._client.TryGetResultStream(result_request)
        result = bytearray()
        valid = True
        for message in streaming_call:
            ret = message.WhichOneof("type")
            if ret is None:
                raise Exception("Error with server")
            elif ret == "result":
                if message.result.WhichOneof("type") == "data":
                    result += message.result.data
                    valid = False
                elif message.result.WhichOneof("type") == "data_complete":
                    valid = True
            elif ret == "error":
                raise Exception("Task in error")
            else:
                raise Exception("Unknown return type")
        if valid:
            return result
        raise Exception("Incomplete Data")

    def request_output_id(self, session_id: str) -> str:
        return f"{session_id}%{uuid.uuid4()}"


def to_request_stream_internal(request, is_last, chunk_max_size):
    req = CreateLargeTaskRequest(
        init_task=InitTaskRequest(
            header=TaskRequestHeader(
                data_dependencies=request.data_dependencies,
                expected_output_keys=request.expected_output_keys
            )
        )
    )
    yield req
    start = 0
    payload_length = len(request.payload)
    if payload_length == 0:
        req = CreateLargeTaskRequest(
            task_payload=DataChunk(data=b'')
        )
        yield req
    while start < payload_length:
        chunk_size = min(chunk_max_size, payload_length - start)
        req = CreateLargeTaskRequest(
            task_payload=DataChunk(data=request.payload[start:start + chunk_size])
        )
        yield req
        start += chunk_size
    req = CreateLargeTaskRequest(
        task_payload=DataChunk(data_complete=True)
    )
    yield req

    if is_last:
        req = CreateLargeTaskRequest(
            init_task=InitTaskRequest(last_task=True)
        )
        yield req


def to_request_stream(requests, s_id, t_options, chunk_max_size):
    req = CreateLargeTaskRequest(
        init_request=CreateLargeTaskRequest.InitRequest(
            session_id=s_id, task_options=t_options))
    yield req
    if len(requests) == 0:
        return
    for r in requests[:-1]:
        for req in to_request_stream_internal(r, False, chunk_max_size):
            yield req
    for req in to_request_stream_internal(requests[-1], True, chunk_max_size):
        yield req
