import uuid
from typing import Optional, List, Tuple, Dict, Union, Generator

from grpc import Channel

from ..common import get_task_filter, TaskOptions, TaskDefinition, Task, TaskStatus, ResultAvailability
from ..protogen.client.submitter_service_pb2_grpc import SubmitterStub
from ..protogen.common.objects_pb2 import Empty, TaskRequest, ResultRequest, DataChunk, InitTaskRequest, \
    TaskRequestHeader, Configuration, Session, TaskOptions as InnerTaskOptions
from ..protogen.common.submitter_common_pb2 import CreateSessionRequest, GetTaskStatusRequest, CreateLargeTaskRequest, \
    WaitRequest


class ArmoniKSubmitter:
    def __init__(self, grpc_channel: Channel):
        """Create a Submitter with the given channel

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = SubmitterStub(grpc_channel)

    def get_service_configuration(self) -> Configuration:
        """Get service configuration

        Returns:
            Configuration object containing the chunk size
        """
        return self._client.GetServiceConfiguration(Empty())

    def create_session(self, default_task_options: TaskOptions, partition_ids: Optional[List[str]] = None) -> str:
        """Create a session

        Args:
            default_task_options: Default TaskOptions used when
                submitting tasks without specifying the options
            partition_ids: List of partitions this session can send
                tasks to. If unspecified, can only send to the default
                partition

        Returns:
            Session Id
        """
        if partition_ids is None:
            partition_ids = []
        request = CreateSessionRequest(default_task_option=default_task_options.to_message())
        for partition in partition_ids:
            request.partition_ids.append(partition)
        return self._client.CreateSession(request).session_id

    def cancel_session(self, session_id: str) -> None:
        """Cancel a session

        Args:
            session_id: Id of the session to b cancelled
        """
        self._client.CancelSession(Session(id=session_id))

    def submit(self, session_id: str, tasks: List[TaskDefinition], task_options: Optional[TaskOptions] = None) -> Tuple[List[Task], List[str]]:
        """Send tasks to ArmoniK

        Args:
            session_id: Session Id
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
            if t.data_dependencies:
                task_request.data_dependencies.extend(t.data_dependencies)
            task_request.payload = t.payload
            task_requests.append(task_request)

        configuration = self.get_service_configuration()
        create_tasks_reply = self._client.CreateLargeTasks(
            _to_request_stream(task_requests, session_id, task_options.to_message() if task_options is not None else None, configuration.data_chunk_max_size))
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
        """List tasks

        Args:
            session_ids: List of session ids from which to list tasks
                from. Mutually exclusive with task_ids
            task_ids: List of task ids to list. Mutually exclusive with
                session_ids
            included_statuses: List of statuses to list tasks from,
                excluding other stask statuses. Mutually exclusive with
                excluded_statuses
            excluded_statuses: List of statuses to not list tasks from,
                including other stask statuses. Mutually exclusive with
                included_statuses

        Returns:
            List of task ids
        """
        return [t for t in self._client.ListTasks(
            get_task_filter(session_ids, task_ids, included_statuses, excluded_statuses)).task_ids]

    def get_task_status(self, task_ids: List[str]) -> Dict[str, TaskStatus]:
        """Get statuses of a given task list

        Args:
            task_ids: List of task ids

        Returns:
            Dictionary mapping a task id to the status of the
            corresponding task
        """
        request = GetTaskStatusRequest()
        request.task_ids.extend(task_ids)
        reply = self._client.GetTaskStatus(request)
        return {s.task_id: TaskStatus(s.status) for s in reply.id_statuses}

    def wait_for_completion(self,
                            session_ids: Optional[List[str]] = None,
                            task_ids: Optional[List[str]] = None,
                            included_statuses: Optional[List[TaskStatus]] = None,
                            excluded_statuses: Optional[List[TaskStatus]] = None,
                            stop_on_first_task_error: bool = False,
                            stop_on_first_task_cancellation: bool = False) -> Dict[TaskStatus, int]:
        """Wait for the tasks matching the filters

        Args:
            session_ids: List of session ids from which to list tasks
                from. Mutually exclusive with task_ids
            task_ids: List of task ids to list. Mutually exclusive with
                session_ids
            included_statuses: List of statuses to list tasks from,
                excluding other stask statuses. Mutually exclusive with
                excluded_statuses
            excluded_statuses: List of statuses to not list tasks from,
                including other stask statuses. Mutually exclusive with
                included_statuses
            stop_on_first_task_error: If set to true, stop the wait if a
                matching task fails
            stop_on_first_task_cancellation: If set to true, stop the
                wait if a matching task is cancelled

        Returns:
            Dictionary containing the number of tasks in each status
            after waiting for completion
        """
        return {TaskStatus(sc.status): sc.count for sc in self._client.WaitForCompletion(
            WaitRequest(filter=get_task_filter(session_ids, task_ids, included_statuses, excluded_statuses),
                        stop_on_first_task_error=stop_on_first_task_error,
                        stop_on_first_task_cancellation=stop_on_first_task_cancellation)).values}

    def get_result(self, session_id: str, result_id: str) -> Union[bytes, None]:
        """Get a result

        Args:
            session_id: Session Id
            result_id: Result Id

        Returns:
            content of the result as bytes
        """
        result_request = ResultRequest(
            result_id=result_id,
            session=session_id
        )
        streaming_call = self._client.TryGetResultStream(result_request)
        result = bytearray()
        valid = False
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
            elif ret == "not_completed_task":
                return None
            else:
                raise Exception("Unknown return type")
        if valid:
            return result
        raise Exception("Incomplete Data")

    def wait_for_availability(self, session_id: str, result_id: str) -> Union[ResultAvailability, None]:
        """Blocks until the result is available or is in error

        Args:
            session_id: Session Id
            result_id: Result Id

        Returns:
            None if the wait was cancelled unexpectedly, otherwise a
            ResultAvailability with potential errors
        """
        result_request = ResultRequest(
            result_id=result_id,
            session=session_id
        )
        response = self._client.WaitForAvailability(result_request)
        response_type = response.WhichOneof("type")
        if response_type == "ok":
            return ResultAvailability()
        if response_type == "error":
            return ResultAvailability(errors=[e.detail for e in response.error.errors])
        return None


def _to_request_stream_internal(request: TaskRequest, is_last: bool, chunk_max_size: int) -> Generator[CreateLargeTaskRequest, None, None]:
    """ Generate the CreateLargeTaskRequests for the given request

    Args:
        request: TaskRequest
        is_last: true if the request is the last one to send
        chunk_max_size: Chunk max size

    Yields:
        CreateLargeTaskRequest
    """
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


def _to_request_stream(requests: List[TaskRequest], s_id: str, t_options: Optional[InnerTaskOptions], chunk_max_size: int) -> Generator[CreateLargeTaskRequest, None, None]:
    """ Generate the CreateLargeTaskRequests from a list of TaskRequest
    This is necessary to respect the payload size limit
    Args:
        requests: List of Task requests
        s_id: session id
        t_options: Task options
        chunk_max_size: chunk max size

    Yields:
        CreateLargeTaskRequest from the list of task requests
    """
    req = CreateLargeTaskRequest(
        init_request=CreateLargeTaskRequest.InitRequest(
            session_id=s_id, task_options=t_options))
    yield req
    if len(requests) == 0:
        return
    for r in requests[:-1]:
        for req in _to_request_stream_internal(r, False, chunk_max_size):
            yield req
    for req in _to_request_stream_internal(requests[-1], True, chunk_max_size):
        yield req
