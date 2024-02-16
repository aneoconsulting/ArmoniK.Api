import numpy as np

from .base import ArmoniKMetric
from ...common import Task, TaskTimestamps


class TimestampsTransition(ArmoniKMetric):
    """
    Metric to compute statistics on transitions between two timestamps in tasks.
    """

    def __init__(self, timestamp_1: str, timestamp_2: str) -> None:
        """
        Initialize the metric.

        Args:
            timestamp_1 (str): The first timestamp.
            timestamp_2 (str): The second timestamp.
        """
        self.timestamps = (timestamp_1, timestamp_2)
        self.avg = 0
        self.min = None
        self.max = None
        self.__class__.__qualname__ = (
            f"{self.timestamps[0].capitalize()}To{self.timestamps[1].capitalize()}"
        )

    @property
    def timestamps(self) -> tuple[str, str]:
        """
        Get the timestamps.

        Returns:
            tuple[str, str]: A tuple containing two timestamps.
        """
        return self.__timestamps

    @timestamps.setter
    def timestamps(self, __value: tuple[str, str]) -> None:
        """
        Set the timestamps.

        Args:
            __value (tuple[str, str]): A tuple containing two timestamps.

        Raises:
            ValueError: If the timestamps are not valid or in inconsistent order.
        """
        for timestamp in __value:
            if not TaskTimestamps.has_value(timestamp):
                raise ValueError(f"{timestamp} is not a valid timestamp.")
        if getattr(TaskTimestamps, __value[0].upper()) > getattr(
            TaskTimestamps, __value[1].upper()
        ):
            raise ValueError(
                f"Inconsistent timestamp order '{__value[0]}' is not prior to '{__value[1]}'."
            )
        self.__timestamps = __value

    def update(self, total: int, tasks: list[Task]) -> None:
        """
        Update the metric with new data.
        Update the average, minimum, and maximum transition times between the two timestamps.

        Args:
            total (int): Total number of tasks.
            tasks (list[Task]): List of tasks.
        """
        deltas = [
            (
                getattr(t, f"{self.timestamps[1]}_at") - getattr(t, f"{self.timestamps[0]}_at")
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

    @property
    def values(self) -> dict[str, float]:
        """
        Get the computed values.

        Returns:
            dict[str, float]: A dictionary containing the average, minimum, and maximum transition times.
        """
        return {"avg": self.avg, "min": self.min, "max": self.max}
