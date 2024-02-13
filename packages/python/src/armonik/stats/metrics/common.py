from datetime import datetime

from .base import ArmoniKMetric
from ...common import Task


class TotalElapsedTime(ArmoniKMetric):
    """
    A metric to compute the total elapsed time between the first task and the last task.
    """
    def __init__(self) -> None:
        self.elapsed = None

    def complete(self, start: datetime, end: datetime) -> None:
        """
        Calculate the total elapsed time.

        Args:
            start (datetime): The start time.
            end (datetime): The end time.
        """
        self.elapsed = (end - start).total_seconds()

    @property
    def values(self) -> float:
        """
        Return the total elapsed time as the metric value.

        Return:
            int: The total elasped time.
        """
        return self.elapsed


class AvgThroughput(ArmoniKMetric):
    """
    A metric to compute the average throughput.
    """
    def __init__(self) -> None:
        self.throughput = None
        self.total = None

    def update(self, total: int, tasks: list[Task]) -> None:
        """
        Update the total number of tasks.

        Args:
            total (int): Total number of tasks.
            tasks (list[Task]): A task batch.
        """
        self.total = total

    def complete(self, start: datetime, end: datetime) -> None:
        """
        Calculate the average throughput.

        Args:
            start (datetime): The start time.
            end (datetime): The end time.
        """
        self.throughput = self.total / (end - start).total_seconds()

    @property
    def values(self) -> int:
        """
        Return the average throughput as the metric value.

        Return:
            int: The average throughput.
        """
        return self.throughput
