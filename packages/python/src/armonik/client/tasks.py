from grpc import Channel

from ..common.objects import Task, TaskStatus, TaskOptions
from ..protogen.client.tasks_service_pb2_grpc import TasksStub
from ..protogen.common.tasks_common_pb2 import GetTaskRequest


class ArmoniKTasks:
    def __init__(self, grpc_channel: Channel):
        self._client = TasksStub(grpc_channel)

    def get_task(self, task_id: str) -> Task:
        task_response = self._client.GetTask(GetTaskRequest(task_id=task_id))
        task = Task()
        raw = task_response.task
        task.id = raw.id
        task.session_id = raw.session_id
        task.owner_pod_id = raw.owner_pod_id
        task.parent_task_ids.extend(raw.parent_task_ids)
        task.data_dependencies.extend(raw.data_dependencies)
        task.expected_output_ids.extend(raw.expected_output_ids)
        task.status = TaskStatus(raw.status)
        task.status_message = raw.status_message
        task.options = TaskOptions.from_message(raw.options)
        task.retry_of_ids.extend(raw.retry_of_ids)
        return task
