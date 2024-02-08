from datetime import datetime, timezone
from armonik.common.filter import Filter
from armonik.protogen.common.sort_direction_pb2 import SortDirection

import numpy as np
import pytest

from .conftest import get_client
from armonik.client import ArmoniKTasks, TaskFieldFilter
from armonik.common import Direction, Task
from armonik.utils import ArmoniKStatistics
from armonik.utils.stats import (
    TotalElapsedTime,
    ArmoniKStatItem,
    AvgThroughput,
    StatusTransition,
    TasksInStatusOverTime,
)


class DummyStatItem(ArmoniKStatItem):
    def __init__(self):
        super().__init__("dummy")
        self.updates = 0
        self.finalizes = 0
        self.total = None
        self.tasks = []
        self.start = None
        self.end = None

    def update(self, total: int, tasks: list[Task]):
        self.updates += 1
        self.total = total
        self.tasks.append(tasks)

    def finalize(self, start: datetime, end: datetime):
        self.start = start
        self.end = end
        self.finalizes += 1

    @property
    def values(self):
        return "value"


task_batch_1 = [
    Task(
        id=i,
        created_at=datetime(1, 1, 1, 1, 1, 1, tzinfo=timezone.utc),
        submitted_at=datetime(1, 1, 1, 1, 1, 1, 1 + i, tzinfo=timezone.utc),
        ended_at=datetime(1, 1, 1, 1, 1, 1 + i, tzinfo=timezone.utc),
    )
    for i in range(3)
]
task_batch_2 = [
    Task(
        id=i,
        created_at=datetime(1, 1, 1, 1, 1, 0, tzinfo=timezone.utc),
        submitted_at=datetime(1, 1, 1, 1, 1, 0, 1 + i, tzinfo=timezone.utc),
        ended_at=datetime(1, 1, 1, 1, 1, 1 + i, tzinfo=timezone.utc),
    )
    for i in range(3, 5)
]
start = task_batch_2[0].created_at
end = task_batch_2[-1].ended_at


class DummyArmoniKTasks(ArmoniKTasks):
    __call_count = 0

    def list_tasks(
        self,
        task_filter: Filter | None = None,
        with_errors: bool = False,
        page: int = 0,
        page_size: int = 1000,
        sort_field: Filter = TaskFieldFilter.TASK_ID,
        sort_direction: SortDirection = Direction.ASC,
        detailed: bool = True,
    ) -> tuple[int, list[Task]]:
        self.__call_count += 1
        if self.__call_count == 1:
            return 5, task_batch_1
        elif self.__call_count == 2:
            return 5, task_batch_2
        return 5, []


class TestArmoniKStatistics:
    def test_constructor(self):
        channel = get_client("Channel")

        ArmoniKStatistics(
            channel=channel,
            task_filter=TaskFieldFilter.SESSION_ID == "session-id",
            stat_items=[TotalElapsedTime()],
        )

        with pytest.raises(TypeError):
            ArmoniKStatistics(
                channel=channel,
                task_filter=TaskFieldFilter.SESSION_ID == "session-id",
                stat_items="a",
            )

        with pytest.raises(TypeError):
            ArmoniKStatistics(
                channel=channel,
                task_filter=TaskFieldFilter.SESSION_ID == "session-id",
                stat_items=[],
            )

        with pytest.raises(TypeError):
            ArmoniKStatistics(
                channel=channel,
                task_filter=TaskFieldFilter.SESSION_ID == "session-id",
                stat_items=["a", TotalElapsedTime()],
            )

        with pytest.raises(TypeError):
            ArmoniKStatistics(
                channel=channel,
                task_filter=TaskFieldFilter.SESSION_ID == "session-id",
                stat_items=[TotalElapsedTime],
            )

    def test_compute(self):
        channel = get_client("Channel")
        dummy = DummyStatItem()
        stats = ArmoniKStatistics(
            channel=channel,
            task_filter=TaskFieldFilter.SESSION_ID == "session-id",
            stat_items=[dummy],
        )
        stats.client = DummyArmoniKTasks(channel)
        stats.compute()

        assert dummy.updates == 2
        assert dummy.finalizes == 1
        assert stats.values == {"dummy": "value"}
        assert dummy.total == 5
        assert dummy.tasks[0] == task_batch_1 and dummy.tasks[1] == task_batch_2
        assert dummy.start == datetime(1, 1, 1, 1, 1, 0, tzinfo=timezone.utc)
        assert dummy.end == datetime(1, 1, 1, 1, 1, 5, tzinfo=timezone.utc)


class TestAvgThroughput:
    def test_avg_throughput(self):
        th = AvgThroughput()
        th.update(2, task_batch_2)
        th.finalize(start, end)
        assert th.values == 2.0 / 5.0


class TestTotalElapsedTime:
    def test_total_elapsed_time(self):
        tet = TotalElapsedTime()
        tet.update(5, task_batch_1)
        tet.finalize(start, end)
        assert tet.values == 5.0


class TestStatusTransition:
    def test_constructor(self):
        StatusTransition("created", "submitted")

        with pytest.raises(ValueError):
            StatusTransition("created", "wrong")

        with pytest.raises(ValueError):
            StatusTransition("submitted", "created")

    def test_status_transition(self):
        st = StatusTransition("created", "ended")
        st.update(5, task_batch_1)
        st.update(5, task_batch_2)
        st.finalize(start, end)
        assert st.values == {"avg": 12.0 / 5.0, "min": 0.0, "max": 5.0}


class TestTasksInStatusOverTime:
    def test_task_in_status_over_time_no_next_status(self):
        tisot = TasksInStatusOverTime(status="ended")
        tisot.update(5, task_batch_1)
        tisot.update(5, task_batch_2)
        tisot.finalize(start, end)
        assert np.array_equal(
            tisot.values,
            np.array(
                [
                    [
                        start,
                        task_batch_1[0].ended_at,
                        task_batch_1[1].ended_at,
                        task_batch_1[2].ended_at,
                        task_batch_2[0].ended_at,
                        task_batch_2[1].ended_at,
                    ],
                    [0, 1, 2, 3, 4, 5],
                ]
            ),
        )

    def test_task_in_status_over_time_with_next_status(self):
        tisot = TasksInStatusOverTime(status="created", next_status="submitted")
        tisot.update(5, task_batch_1)
        tisot.update(5, task_batch_2)
        tisot.finalize(start, end)
        assert np.array_equal(
            tisot.values,
            np.array(
                [
                    [
                        datetime(1, 1, 1, 1, 1, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, 0, 4, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, 0, 5, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, 1, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, 1, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, 1, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, 1, 1, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, 1, 2, tzinfo=timezone.utc),
                        datetime(1, 1, 1, 1, 1, 1, 3, tzinfo=timezone.utc),
                    ],
                    [0, 1, 2, 1, 0, 1, 2, 3, 2, 1, 0],
                ]
            ),
        )
