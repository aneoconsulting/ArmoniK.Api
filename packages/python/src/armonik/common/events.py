from __future__ import annotations
from typing import List

from dataclasses import dataclass

from .enumwrapper import TaskStatus, ResultStatus


class Event:
    @classmethod
    def from_raw_event(cls, raw_event):
        values = {}
        for raw_field in cls.__annotations__.keys():
            values[raw_field] = getattr(raw_event, raw_field)
        return cls(**values)


@dataclass
class TaskStatusUpdateEvent(Event):
    task_id: str
    status: TaskStatus


@dataclass
class ResultStatusUpdateEvent(Event):
    result_id: str
    status: ResultStatus


@dataclass
class ResultOwnerUpdateEvent(Event):
    result_id: str
    previous_owner_id: str
    current_owner_id: str


@dataclass
class NewTaskEvent(Event):
    task_id: str
    payload_id: str
    origin_task_id: str
    status: TaskStatus
    expected_output_keys: List[str]
    data_dependencies: List[str]
    retry_of_ids: List[str]
    parent_task_ids: List[str]


@dataclass
class NewResultEvent(Event):
    result_id: str
    owner_id: str
    status: ResultStatus
