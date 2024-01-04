import datetime

from .conftest import all_rpc_called, rpc_called, get_client
from armonik.client import ArmoniKTasks, TaskFieldFilter
from armonik.common import Task, TaskDefinition, TaskOptions, TaskStatus, Output


class TestArmoniKTasks:

    def test_get_task(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        task = tasks_client.get_task("task-id")

        assert rpc_called("Tasks", "GetTask")
        assert isinstance(task, Task)
        assert task.id == 'task-id'
        assert task.session_id == 'session-id'
        assert task.data_dependencies == []
        assert task.expected_output_ids == []
        assert task.retry_of_ids == []
        assert task.status == TaskStatus.COMPLETED
        assert task.payload_id is None
        assert task.status_message == ''
        assert task.options == TaskOptions(
            max_duration=datetime.timedelta(seconds=1),
            priority=1,
            max_retries=1,
            partition_id='partition-id',
            application_name='application-name',
            application_version='application-version',
            application_namespace='application-namespace',
            application_service='application-service',
            engine_type='engine-type',
            options={}
        )
        assert task.created_at == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)
        assert task.submitted_at == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)
        assert task.started_at == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)
        assert task.ended_at == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)
        assert task.pod_ttl == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)
        assert task.output == Output(error='')
        assert task.pod_hostname == ''
        assert task.received_at == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)
        assert task.acquired_at == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)

    def test_list_tasks_detailed_no_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        num, tasks = tasks_client.list_tasks()
        assert rpc_called("Tasks", "ListTasksDetailed")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert tasks == []

    def test_list_tasks_detailed_with_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        num, tasks = tasks_client.list_tasks(TaskFieldFilter.STATUS == TaskStatus.COMPLETED)
        assert rpc_called("Tasks", "ListTasksDetailed", 2)
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert tasks == []

    def test_list_tasks_no_detailed_no_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        num, tasks = tasks_client.list_tasks(detailed=False)
        assert rpc_called("Tasks", "ListTasks")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert tasks == []

    def test_cancel_tasks(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        tasks = tasks_client.cancel_tasks(["task-id-1", "task-id-2"])

        assert rpc_called("Tasks", "CancelTasks")
        assert tasks is None

    def test_get_result_ids(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        tasks_results = tasks_client.get_result_ids(["task-id-1", "task-id-2"])
        assert rpc_called("Tasks", "GetResultIds")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert tasks_results == {}

    def test_count_tasks_by_status_no_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        count = tasks_client.count_tasks_by_status()
        assert rpc_called("Tasks", "CountTasksByStatus")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert count == {}

    def test_count_tasks_by_status_with_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        count = tasks_client.count_tasks_by_status(TaskFieldFilter.STATUS == TaskStatus.COMPLETED)
        assert rpc_called("Tasks", "CountTasksByStatus", 2)
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert count == {}

    def test_submit_tasks(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        tasks = tasks_client.submit_tasks(
            "session-id",
            [TaskDefinition(payload_id="payload-id",
                            expected_output_ids=["result-id"],
                            data_dependencies=[],
                            options=TaskOptions(
                                max_duration=datetime.timedelta(seconds=1),
                                priority=1,
                                max_retries=1
                            )
                )
            ],
            default_task_options=TaskOptions(
                max_duration=datetime.timedelta(seconds=1),
                priority=1,
                max_retries=1
            )
        )
        assert rpc_called("Tasks", "SubmitTasks")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert tasks is None

    def test_service_fully_implemented(self):
        assert all_rpc_called("Tasks")
