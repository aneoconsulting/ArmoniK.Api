import abc
from datetime import datetime
from armonik.common import Task

import numpy as np
from grpc import Channel

from ..client import ArmoniKTasks
from ..common import Filter


class ArmoniKStatItem(abc.ABC):
    def __init__(self, name: str) -> None:
        self.name = name

    @abc.abstractmethod
    def update(self, total: int, tasks: list[Task]):
        pass

    @abc.abstractmethod
    def finalize(self, start: datetime, end: datetime):
        pass

    @abc.abstractproperty
    def values(self):
        pass


class TotalElapsedTime(ArmoniKStatItem):
    def __init__(self) -> None:
        super().__init__("total_elapsed_time")
        self.elapsed = None

    def update(self, total: int, tasks: list[Task]):
        pass

    def finalize(self, start: datetime, end: datetime):
        self.elapsed = (end - start).total_seconds()

    @property
    def values(self):
        return self.elapsed


class AvgThroughput(ArmoniKStatItem):
    def __init__(self) -> None:
        super().__init__("avg_throughput")
        self.throughput = None
        self.total = None

    def update(self, total: int, tasks: list[Task]):
        self.total = total

    def finalize(self, start: datetime, end: datetime):
        self.throughput = float(self.total) / (end - start).total_seconds()

    @property
    def values(self):
        return self.throughput


class StatusTransition(ArmoniKStatItem):
    __allowed_status = [
        "created",
        "submitted",
        "received",
        "acquired",
        "started",
        "processed",
        "ended",
    ]

    def __init__(self, status_1: str, status_2: str) -> None:
        super().__init__(f"{status_1}_to_{status_2}")
        self.status = (status_1, status_2)
        self.avg = 0
        self.min = None
        self.max = None

    @property
    def status(self) -> tuple[str, str]:
        return self.__status

    @status.setter
    def status(self, __value: tuple[str, str]) -> None:
        for status in __value:
            if status not in self.__allowed_status:
                raise ValueError(f"{status} is not a valid status.")
        if self.__allowed_status.index(__value[0]) > self.__allowed_status.index(__value[1]):
            raise ValueError(
                f"Inconsistent status order '{status[0]}' is not prior to '{status[1]}'."
            )
        self.__status = __value

    def update(self, total: int, tasks: list[Task]):
        deltas = [
            (
                getattr(t, f"{self.status[1]}_at") - getattr(t, f"{self.status[0]}_at")
            ).total_seconds()
            for t in tasks
        ]
        self.avg += np.sum(deltas) / total
        min = np.min(deltas)
        max = np.max(deltas)
        if self.max is None or self.max < max:
            self.max = max
        if self.min is None or self.min > min:
            self.min = min

    def finalize(self, start: datetime, end: datetime):
        pass

    @property
    def values(self):
        return {"avg": self.avg, "min": self.min, "max": self.max}


class TasksInStatusOverTime(ArmoniKStatItem):
    def __init__(self, status, next_status=None) -> None:
        super().__init__(f"{status}_over_time")
        self.status = status
        self.next_status = next_status
        self.timestamps = None
        self.index = 0

    def update(self, total: int, tasks: list[Task]) -> None:
        n_tasks = len(tasks)
        if self.timestamps is None:
            n = total * 2 + 1 if self.next_status else total + 1
            self.timestamps = np.memmap("timestamps.dat", dtype=object, mode="w+", shape=(2, n))
            self.index = 1
        self.timestamps[:, self.index : self.index + n_tasks] = [
            [getattr(t, f"{self.status}_at") for t in tasks],
            n_tasks * [1],
        ]
        self.index += n_tasks
        if self.next_status:
            self.timestamps[:, self.index : self.index + n_tasks] = [
                [getattr(t, f"{self.next_status}_at") for t in tasks],
                n_tasks * [-1],
            ]
            self.index += n_tasks

    def finalize(self, start: datetime, end: datetime) -> None:
        self.timestamps[:, 0] = (start, 0)
        self.timestamps = self.timestamps[:, self.timestamps[0, :].argsort()]
        self.timestamps[1, :] = np.cumsum(self.timestamps[1, :])

    @property
    def values(self):
        return self.timestamps


class ArmoniKStatistics:
    """A class for computing statistics on ArmoniK tasks.

    Args:
        channel (Channel): gRPC channel to an ArmoniK cluster.
        filter (Filter): Filter to select the tasks on which to compute statistics.
        status_tuples (list[tuple[str, str]]): List of status transitions to compute statistics.
    """

    def __init__(
        self, channel: Channel, task_filter: Filter, stat_items: list[ArmoniKStatItem]
    ) -> None:
        self.client = ArmoniKTasks(channel)
        self.filter = task_filter
        self.stat_items = stat_items

    @property
    def stat_items(self):
        return self.__stat_items

    @stat_items.setter
    def stat_items(self, __value: list[ArmoniKStatItem]):
        if (
            not isinstance(__value, list)
            or len(__value) == 0
            or any([not isinstance(item, ArmoniKStatItem) for item in __value])
        ):
            raise TypeError("'stat_items' must be a list of 'ArmoniKStatItem'.")
        self.__stat_items = __value

    def compute(self):
        """Compute statistics for ArmoniK tasks."""
        start = None
        end = None
        page = 0
        total, tasks = self.client.list_tasks(task_filter=self.filter, page=page)
        while tasks:
            for stat_item in self.stat_items:
                stat_item.update(total, tasks)
            min_start = np.min([t.created_at for t in tasks])
            max_end = np.max([t.ended_at for t in tasks])
            if start is None or min_start < start:
                start = min_start
            if end is None or max_end > end:
                end = max_end
            page += 1
            _, tasks = self.client.list_tasks(task_filter=self.filter, page=page)

        for stat_item in self.stat_items:
            stat_item.finalize(start, end)

    @property
    def values(self):
        """Dict[str, Union[float, dict]]: A dictionary containing computed statistics."""
        return {stat_item.name: stat_item.values for stat_item in self.stat_items}
