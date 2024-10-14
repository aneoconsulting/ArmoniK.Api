from __future__ import annotations

import os
from collections.abc import Mapping
from typing import Optional, Dict, List, Union, Iterator

from ..common import TaskOptions, TaskDefinition, Result, Task
from ..common.helpers import batched
from ..protogen.common.agent_common_pb2 import (
    CreateResultsMetaDataRequest,
    CreateResultsMetaDataResponse,
    NotifyResultDataRequest,
    CreateResultsRequest,
    CreateResultsResponse,
    SubmitTasksRequest,
)
from ..protogen.common.objects_pb2 import (
    Configuration,
)
from ..protogen.common.worker_common_pb2 import ProcessRequest
from ..protogen.worker.agent_service_pb2_grpc import AgentStub


class LazyLoadDict(Mapping):
    def __init__(self, data_folder: str, ids: List[str]):
        self.__data_folder = data_folder
        self._data: Dict[str, Optional[bytes]] = {k: None for k in ids}

    def __iter__(self) -> Iterator[str, bytes]:
        for k in self._data.keys():
            yield k, self[k]

    def keys(self):
        # Overridden to prevent loading
        for k in self._data.keys():
            yield k

    def __getitem__(self, __key) -> bytes:
        if __key not in self._data:
            raise KeyError(__key)
        if self._data[__key] is None:
            with open(os.path.join(self.__data_folder, __key), "rb") as f:
                self._data[__key] = f.read()
        return self._data[__key]

    def __len__(self) -> int:
        return len(self._data)


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

        self._payload = None
        self.data_dependencies = LazyLoadDict(self.data_folder, list(request.data_dependencies))

    @property
    def payload(self) -> bytes:
        if self._payload is None:
            with open(os.path.join(self.data_folder, self.payload_id), "rb") as f:
                self._payload = f.read()
        return self._payload

    def submit_tasks(
        self,
        tasks: List[TaskDefinition],
        default_task_options: Optional[TaskOptions] = None,
        batch_size: Optional[int] = 100,
    ) -> List[Task]:
        """Submit tasks to the agent.

        Args:
            tasks: List of task definitions
            default_task_options: Default Task Options used if a task has its options not set
            batch_size: Batch size for submission
        """
        submitted_tasks: List[Task] = []
        for tasks_batch in batched(tasks, batch_size):
            task_creations = []

            for t in tasks_batch:
                task_creation = SubmitTasksRequest.TaskCreation(
                    expected_output_keys=t.expected_output_ids,
                    payload_id=t.payload_id,
                    data_dependencies=t.data_dependencies,
                )
                if t.options:
                    task_creation.task_options = t.options.to_message()
                task_creations.append(task_creation)

            request = SubmitTasksRequest(
                session_id=self.session_id,
                communication_token=self.token,
                task_creations=task_creations,
            )

            if default_task_options:
                request.task_options = (default_task_options.to_message(),)

            submitted_tasks.extend(
                Task(
                    id=t.task_id,
                    expected_output_ids=list(t.expected_output_ids),
                    data_dependencies=list(t.data_dependencies),
                    session_id=self.session_id,
                    payload_id=self.payload_id,
                )
                for t in self._client.SubmitTasks(request).task_infos
            )
        return submitted_tasks

    def send_results(self, results_data: Dict[str, Union[bytes, bytearray]]) -> None:
        """Send results.

        Args:
            results_data: A dictionary mapping each result ID to its data.
        """
        for result_id, result_data in results_data.items():
            with open(os.path.join(self.data_folder, result_id), "wb") as f:
                f.write(result_data)

        request = NotifyResultDataRequest(
            ids=[
                NotifyResultDataRequest.ResultIdentifier(
                    session_id=self.session_id, result_id=result_id
                )
                for result_id in results_data.keys()
            ],
            communication_token=self.token,
        )
        self._client.NotifyResultData(request)

    def create_results_metadata(
        self, result_names: List[str], batch_size: int = 100
    ) -> Dict[str, Result]:
        """
        Create the metadata of multiple results at once.
        Data have to be uploaded separately.

        Args:
            result_names: The names of the results to create.
            batch_size: Batch size for querying.

        Return:
            A dictionary mapping each result name to its result summary.
        """
        results = {}
        for result_names_batch in batched(result_names, batch_size):
            request = CreateResultsMetaDataRequest(
                results=[
                    CreateResultsMetaDataRequest.ResultCreate(name=result_name)
                    for result_name in result_names_batch
                ],
                session_id=self.session_id,
                communication_token=self.token,
            )
            response: CreateResultsMetaDataResponse = self._client.CreateResultsMetaData(request)
            for result_message in response.results:
                results[result_message.name] = Result.from_result_metadata(result_message)
        return results

    def create_results(
        self, results_data: Dict[str, bytes], batch_size: int = 1
    ) -> Dict[str, Result]:
        """Create one result with data included in the request.

        Args:
            results_data: A dictionary mapping the result names to their actual data.
            batch_size: Batch size for querying.

        Return:
            A dictionary mapping each result name to its corresponding result summary.
        """
        results = {}
        for results_ids_batch in batched(results_data.keys(), batch_size):
            request = CreateResultsRequest(
                results=[
                    CreateResultsRequest.ResultCreate(name=name, data=results_data[name])
                    for name in results_ids_batch
                ],
                session_id=self.session_id,
                communication_token=self.token,
            )
            response: CreateResultsResponse = self._client.CreateResults(request)
            for message in response.results:
                results[message.name] = Result.from_result_metadata(message)
        return results
