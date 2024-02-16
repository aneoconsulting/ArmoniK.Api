from datetime import datetime

import numpy as np

from .base import ArmoniKMetric
from ...common import Task


class TasksInStatusOverTime(ArmoniKMetric):
    """
    A metric to track tasks in a particular status over time.
    """

    def __init__(self, timestamp, next_timestamp=None) -> None:
        """
        Initialize the metric.

        Args:
            timestamp (str): The current timestamp of the tasks.
            next_timestamp (str, optional): The next timestamp of the tasks. Defaults to None.
        """
        self.timestamp = timestamp
        self.next_timestamp = next_timestamp
        self.timestamps = None
        self.index = 0

    def update(self, total: int, tasks: list[Task]) -> None:
        """
        Update the metric.

        Args:
            total (int): Total number of tasks.
            tasks (list[Task]): A task batch.
        """
        n_tasks = len(tasks)
        if self.timestamps is None:
            n = total * 2 + 1 if self.next_timestamp else total + 1
            self.timestamps = np.memmap("timestamps.dat", dtype=object, mode="w+", shape=(2, n))
            self.index = 1
        self.timestamps[:, self.index : self.index + n_tasks] = [
            [getattr(t, f"{self.timestamp}_at") for t in tasks],
            n_tasks * [1],
        ]
        self.index += n_tasks
        if self.next_timestamp:
            self.timestamps[:, self.index : self.index + n_tasks] = [
                [getattr(t, f"{self.next_timestamp}_at") for t in tasks],
                n_tasks * [-1],
            ]
            self.index += n_tasks

    def complete(self, start: datetime, end: datetime) -> None:
        """
        Complete the metric calculation.

        Args:
            start (datetime): The start time.
            end (datetime): The end time.
        """
        self.timestamps[:, 0] = (start, 0)
        self.timestamps = self.timestamps[:, self.timestamps[0, :].argsort()]
        self.timestamps[1, :] = np.cumsum(self.timestamps[1, :])

    @property
    def values(self):
        """
        Return the timestamps as the metric values.
        """
        return self.timestamps
