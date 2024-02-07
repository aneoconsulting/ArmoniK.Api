import datetime
from typing import List, Tuple
from armonik.common.filter import Filter
from armonik.protogen.common.sort_direction_pb2 import SortDirection

import pytest

from grpc import Channel

from .conftest import get_client
from armonik.client import ArmoniKTasks, TaskFieldFilter
from armonik.common import Direction, Task
from armonik.utils import ArmoniKStatistics


class DummyArmoniKTasks(ArmoniKTasks):
    __call_count = 0

    __tasks = [
        Task(
            created_at=datetime.datetime(1971, 1, 1, 1, 1, 1, tzinfo=datetime.timezone.utc),
            submitted_at=datetime.datetime(1971, 1, 1, 1, 1, 2, tzinfo=datetime.timezone.utc),
            received_at=datetime.datetime(1971, 1, 1, 1, 1, 3, tzinfo=datetime.timezone.utc),
            ended_at=datetime.datetime(1971, 1, 1, 1, 1, 4, tzinfo=datetime.timezone.utc),
        ),
        Task(
            created_at=datetime.datetime(1971, 1, 1, 1, 1, 2, tzinfo=datetime.timezone.utc),
            submitted_at=datetime.datetime(1971, 1, 1, 1, 1, 3, tzinfo=datetime.timezone.utc),
            received_at=datetime.datetime(1971, 1, 1, 1, 1, 4, tzinfo=datetime.timezone.utc),
            ended_at=datetime.datetime(1971, 1, 1, 1, 1, 5, tzinfo=datetime.timezone.utc),
        ),
        Task(
            created_at=datetime.datetime(1971, 1, 1, 1, 1, 4, tzinfo=datetime.timezone.utc),
            submitted_at=datetime.datetime(1971, 1, 1, 1, 1, 5, tzinfo=datetime.timezone.utc),
            received_at=datetime.datetime(1971, 1, 1, 1, 1, 6, tzinfo=datetime.timezone.utc),
            ended_at=datetime.datetime(1971, 1, 1, 1, 1, 9, tzinfo=datetime.timezone.utc),
        ),
    ]


    def list_tasks(self, task_filter: Filter | None = None, with_errors: bool = False, page: int = 0, page_size: int = 1000, sort_field: Filter = TaskFieldFilter.TASK_ID, sort_direction: SortDirection = Direction.ASC, detailed: bool = True) -> Tuple[int, List[Task]]:
        self.__call_count += 1
        if self.__call_count == 1:
            return 3, self.__tasks
        return 3, []


class TestArmoniKStatistics:

    def test_constructor(self):
        channel = get_client("Channel")

        ArmoniKStatistics(channel=channel, task_filter=TaskFieldFilter.SESSION_ID == "session-id", status_tuples=[("created", "submitted")])

        with pytest.raises(TypeError):
            ArmoniKStatistics(channel=channel, task_filter=TaskFieldFilter.SESSION_ID == "session-id", status_tuples=("created", "submitted"))

        with pytest.raises(TypeError):
            ArmoniKStatistics(channel=channel, task_filter=TaskFieldFilter.SESSION_ID == "session-id", status_tuples=[("created", "submitted", "received")])

        with pytest.raises(ValueError):
            ArmoniKStatistics(channel=channel, task_filter=TaskFieldFilter.SESSION_ID == "session-id", status_tuples=[("wrong", "created")])

        with pytest.raises(ValueError):
            ArmoniKStatistics(channel=channel, task_filter=TaskFieldFilter.SESSION_ID == "session-id", status_tuples=[("submitted", "created")])

    def test_compute(self):
        channel = get_client("Channel")
        stats = ArmoniKStatistics(channel=channel, task_filter=TaskFieldFilter.SESSION_ID == "session-id", status_tuples=[("created", "submitted"), ("received", "ended")])
        stats.client = DummyArmoniKTasks(channel)
        stats.compute()

        assert stats.values["total_elapsed_time"] == 8.
        assert stats.values["throughput"] == 3. / 8.
        assert stats.values["created_to_submitted"] == {"avg": 1., "min": 1., "max": 1.}
        assert stats.values["received_to_ended"] == {"avg": 5. / 3., "min": 1., "max": 3.}
