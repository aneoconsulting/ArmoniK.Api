import numpy as np
from grpc import Channel

from .metrics import ArmoniKMetric
from ..client import ArmoniKTasks
from ..common import Filter


class ArmoniKStatistics:
    """A class for computing statistics on ArmoniK tasks.

    Args:
        channel (Channel): gRPC channel to an ArmoniK cluster.
        filter (Filter): Filter to select the tasks on which to compute statistics.
        status_tuples (list[tuple[str, str]]): List of status transitions to compute statistics.
    """

    def __init__(self, channel: Channel, task_filter: Filter, metrics: list[ArmoniKMetric]) -> None:
        self.client = ArmoniKTasks(channel)
        self.filter = task_filter
        self.metrics = metrics

    @property
    def metrics(self) -> list[ArmoniKMetric]:
        return self.__metrics

    @metrics.setter
    def metrics(self, __value: list[ArmoniKMetric]) -> None:
        if (
            not isinstance(__value, list)
            or len(__value) == 0
            or any([not isinstance(item, ArmoniKMetric) for item in __value])
        ):
            raise TypeError("'metrics' must be a list of 'ArmoniKMetric'.")
        self.__metrics = __value

    def compute(self) -> None:
        """Compute statistics for ArmoniK tasks."""
        start = None
        end = None
        page = 0
        total, tasks = self.client.list_tasks(task_filter=self.filter, page=page)
        while tasks:
            for metric in self.metrics:
                metric.update(total, tasks)
            min_start = np.min([t.created_at for t in tasks])
            max_end = np.max([t.ended_at for t in tasks])
            if start is None or min_start < start:
                start = min_start
            if end is None or max_end > end:
                end = max_end
            page += 1
            _, tasks = self.client.list_tasks(task_filter=self.filter, page=page)

        for metric in self.metrics:
            metric.complete(start, end)

    @property
    def values(self):
        """Dict[str, Union[float, dict]]: A dictionary containing computed statistics."""
        return {metric.__class__.__qualname__: metric.values for metric in self.metrics}
