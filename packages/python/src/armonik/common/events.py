from abc import ABC
from typing import List

from dataclasses import dataclass, fields

from .enumwrapper import TaskStatus, ResultStatus


@dataclass
class Event(ABC):
    @classmethod
    def from_raw_event(cls, raw_event):
        values = {}
        for raw_field in fields(cls):
            values[raw_field] = getattr(raw_event, raw_field)
        return cls(**values)


class TaskStatusUpdateEvent(Event):
    task_id: str
    status: TaskStatus


class ResultStatusUpdateEvent(Event):
    result_id: str
    status: ResultStatus


class ResultOwnerUpdateEvent(Event):
    result_id: str
    previous_owner_id: str
    current_owner_id: str


class NewTaskEvent(Event):
    task_id: str
    payload_id: str
    origin_task_id: str
    status: TaskStatus
    expected_output_keys: List[str]
    data_dependencies: List[str]
    retry_of_ids: List[str]
    parent_task_ids: List[str]


class NewResultEvent(Event):
    result_id: str
    owner_id: str
    status: ResultStatus
