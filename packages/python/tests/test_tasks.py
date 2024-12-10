import datetime

from .conftest import all_rpc_called, rpc_called, get_client
from armonik.client import ArmoniKTasks, TaskFieldFilter
from armonik.common import Task, TaskDefinition, TaskOptions, TaskStatus, Output


class TestArmoniKTasks:
    mock_task = Task(
        id="task-id",
        session_id="session-id",
        owner_pod_id="",
        initial_task_id="",
        created_by="",
        parent_task_ids=[],
        data_dependencies=[],
        expected_output_ids=[],
        retry_of_ids=[],
        status=4,
        status_message="",
        options=TaskOptions(
            max_duration=datetime.timedelta(seconds=1),
            priority=1,
            max_retries=1,
            partition_id="partition-id",
            application_name="application-name",
            application_version="application-version",
            application_namespace="application-namespace",
            application_service="application-service",
            engine_type="engine-type",
            options={},
        ),
        created_at=None,
        submitted_at=None,
        received_at=None,
        acquired_at=None,
        fetched_at=None,
        started_at=None,
        processed_at=None,
        ended_at=None,
        pod_ttl=None,
        creation_to_end_duration=datetime.timedelta(0),
        processing_to_end_duration=datetime.timedelta(0),
        received_to_end_duration=datetime.timedelta(0),
        output=Output(error=""),
        pod_hostname="",
        payload_id="",
    )

    def test_get_task(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        task = tasks_client.get_task("task-id")

        assert rpc_called("Tasks", "GetTask")
        assert isinstance(task, Task)
        assert task == self.mock_task

    def test_list_tasks_detailed_no_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        num, tasks = tasks_client.list_tasks()
        assert rpc_called("Tasks", "ListTasksDetailed")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 1
        assert tasks == [self.mock_task]

    def test_list_tasks_detailed_with_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        num, tasks = tasks_client.list_tasks(TaskFieldFilter.STATUS == TaskStatus.COMPLETED)
        assert rpc_called("Tasks", "ListTasksDetailed", 2)
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 1
        assert tasks == [self.mock_task]

    def test_list_tasks_no_detailed_no_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        num, tasks = tasks_client.list_tasks(detailed=False)
        assert rpc_called("Tasks", "ListTasks")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 1
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
        assert count == {TaskStatus.COMPLETED: 2, TaskStatus.SUBMITTED: 5}

    def test_count_tasks_by_status_with_filter(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        count = tasks_client.count_tasks_by_status(TaskFieldFilter.STATUS == TaskStatus.COMPLETED)
        assert rpc_called("Tasks", "CountTasksByStatus", 2)
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert count == {TaskStatus.COMPLETED: 2, TaskStatus.SUBMITTED: 5}

    def test_submit_tasks(self):
        tasks_client: ArmoniKTasks = get_client("Tasks")
        tasks = tasks_client.submit_tasks(
            "session-id",
            [
                TaskDefinition(
                    payload_id="payload-id",
                    expected_output_ids=["result-id"],
                    data_dependencies=[],
                    options=TaskOptions(
                        max_duration=datetime.timedelta(seconds=1),
                        priority=1,
                        max_retries=1,
                    ),
                )
            ],
            default_task_options=TaskOptions(
                max_duration=datetime.timedelta(seconds=1), priority=1, max_retries=1
            ),
        )
        assert rpc_called("Tasks", "SubmitTasks")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert tasks is not None

    def test_service_fully_implemented(self):
        assert all_rpc_called("Tasks")
