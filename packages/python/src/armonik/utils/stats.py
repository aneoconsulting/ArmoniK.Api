from grpc import Channel

from ..client import ArmoniKTasks
from ..common import Filter, Task


def getsetattr(obj: object, __name: str):
    if not hasattr(obj, __name):
        setattr(obj, __name, {"avg": 0, "min": None, "max": None})
    return getattr(obj, __name)


class ArmoniKStatistics:

    __real_status = ["created", "submitted", "received", "acquired", "started", "processed", "ended"] #[attr.removesuffix("_at") for attr in dir(Task) if attr.endswith("_at")]

    def __init__(self, channel: Channel, task_filter: Filter, status_tuples: list[tuple[str, str]] = [(state_1, state_2) for state_1, state_2 in zip(__real_status[1:], __real_status[:-1])]) -> None:
        self.client = ArmoniKTasks(channel)
        self.filter = task_filter
        self.status_tuples = status_tuples

    @property
    def status_tuples(self):
        return self._status_tuples

    @status_tuples.setter
    def status_tuples(self, __value: list[tuple[str, str]]):
        if not isinstance(__value, list):
            raise TypeError("'status_tuple' must be a two-element tuple list.")
        for states in __value:
            if len(states) != 2:
                raise TypeError("'status_tuple' must be a two-element tuple list.")
            for state in states:
                if state not in self.__real_status:
                    raise ValueError(f"{state} is not a valid status.")
            if self.__real_status.index(states[0]) > self.__real_status.index(states[1]):
                raise ValueError(f"Inconsistent status order '{states[0]}' is not prior to '{states[1]}'.")
        self._status_tuples = __value

    def compute(self):
        start = None
        end = None

        page = 0
        self.throughput, tasks = self.client.list_tasks(task_filter=self.filter, page=page)
        while tasks:
            for task in tasks:
                for state_1, state_2 in self.status_tuples:
                    property = getsetattr(self, f"{state_1}_to_{state_2}")
                    delta = (getattr(task, f"{state_2}_at") - getattr(task, f"{state_1}_at")).total_seconds()
                    property["avg"] += delta
                    if not property["max"] or property["max"] < delta:
                        property["max"] = delta
                    if not property["min"] or property["min"] > delta:
                        property["min"] = delta

                if not start or task.created_at < start:
                    start = task.created_at
                if not end or task.ended_at > end:
                    end = task.ended_at

            page += 1
            _, tasks = self.client.list_tasks(task_filter=self.filter, page=page)

        for state_1, state_2 in self.status_tuples:
            getattr(self, f"{state_1}_to_{state_2}")["avg"] /= self.throughput

        self.total_elapsed_time = (end - start).total_seconds()

        assert self.total_elapsed_time > 0
        self.throughput /= self.total_elapsed_time

    @property
    def values(self):
        values = {"throughput": self.throughput, "total_elapsed_time": self.total_elapsed_time}
        for state_1, state_2 in self.status_tuples:
            key = f"{state_1}_to_{state_2}"
            value = getattr(self, key)
            values[key] = value
        return values

"""
docstring
cas par défaut
cas timestamp non défini (tâche en erreur)
"""