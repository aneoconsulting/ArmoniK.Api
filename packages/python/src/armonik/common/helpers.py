from datetime import timedelta, datetime
from math import floor
from typing import List, Optional

import google.protobuf.duration_pb2 as duration
import google.protobuf.timestamp_pb2 as timestamp

from ..protogen.common.submitter_common_pb2 import TaskFilter
from .enumwrapper import TaskStatus


def get_task_filter(session_ids: Optional[List[str]] = None, task_ids: Optional[List[str]] = None,
                    included_statuses: Optional[List[TaskStatus]] = None,
                    excluded_statuses: Optional[List[TaskStatus]] = None) -> TaskFilter:
    if session_ids is not None and task_ids is not None:
        raise ValueError("session_ids and task_ids cannot be defined at the same time")
    if included_statuses is not None and excluded_statuses is not None:
        raise ValueError("included_statuses and excluded_statuses cannot be defined at the same time")
    task_filter = TaskFilter(
        session=TaskFilter.IdsRequest() if session_ids is not None else None,
        task=TaskFilter.IdsRequest() if task_ids is not None else None,
        included=TaskFilter.StatusesRequest() if included_statuses is not None else None,
        excluded=TaskFilter.StatusesRequest() if excluded_statuses is not None else None
    )
    if session_ids is not None:
        task_filter.session.ids.extend(session_ids)
    if task_ids is not None:
        task_filter.task.ids.extend(task_ids)
    if included_statuses is not None:
        task_filter.included.statuses.extend([t.value for t in included_statuses])
    if excluded_statuses is not None:
        task_filter.excluded.statuses.extend([t.value for t in excluded_statuses])
    return task_filter


def datetime_to_timestamp(time_stamp: datetime) -> timestamp.Timestamp:
    secs, fracsec = divmod(time_stamp.timestamp(), 1)
    return timestamp.Timestamp(seconds=secs, nanos=floor(fracsec * 1e9))


def timestamp_to_datetime(time_stamp: timestamp.Timestamp) -> datetime:
    return datetime.utcfromtimestamp(time_stamp.seconds + time_stamp.nanos / 1e9)


def duration_to_timedelta(delta: duration.Duration) -> timedelta:
    return timedelta(seconds=delta.seconds, microseconds=delta.nanos // 1000)


def timedelta_to_duration(delta: timedelta) -> duration.Duration:
    secs, remainder = divmod(delta, timedelta(seconds=1))
    return duration.Duration(seconds=secs, nanos=(remainder // timedelta(microseconds=1)) * 1000)
