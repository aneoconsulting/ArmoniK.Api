from __future__ import annotations
from datetime import timedelta, datetime, timezone
from typing import List, Optional, Iterable, TypeVar

import google.protobuf.duration_pb2 as duration
import google.protobuf.timestamp_pb2 as timestamp

from ..protogen.common.submitter_common_pb2 import TaskFilter
from .enumwrapper import TaskStatus


T = TypeVar("T")


def get_task_filter(
    session_ids: Optional[List[str]] = None,
    task_ids: Optional[List[str]] = None,
    included_statuses: Optional[List[TaskStatus]] = None,
    excluded_statuses: Optional[List[TaskStatus]] = None,
) -> TaskFilter:
    """Helper function to generate a task filter from the parameters

    Args:
        session_ids: Optional list of session Ids to filter against, mutually exclusive with task_ids
        task_ids: Optional list of task ids to filter against, mutually exclusive with session_ids
        included_statuses: Optional list of task statuses to include in the filter, mutually exclusive with excluded_statuses
        excluded_statuses: Optional list of task statuses to exclude in the filter, mutually exclusive with included_statuses

    Returns:
        Task filter to be used in a gRPC call to filter tasks
    """
    if session_ids and task_ids:
        raise ValueError("session_ids and task_ids cannot be defined at the same time")
    if included_statuses and excluded_statuses:
        raise ValueError(
            "included_statuses and excluded_statuses cannot be defined at the same time"
        )
    task_filter = TaskFilter(
        session=TaskFilter.IdsRequest() if session_ids else None,
        task=TaskFilter.IdsRequest() if task_ids else None,
        included=TaskFilter.StatusesRequest() if included_statuses else None,
        excluded=TaskFilter.StatusesRequest() if excluded_statuses else None,
    )
    if session_ids:
        task_filter.session.ids.extend(session_ids)
    if task_ids:
        task_filter.task.ids.extend(task_ids)
    if included_statuses:
        task_filter.included.statuses.extend(included_statuses)
    if excluded_statuses:
        task_filter.excluded.statuses.extend(excluded_statuses)
    return task_filter


def datetime_to_timestamp(time_stamp: Optional[datetime]) -> timestamp.Timestamp:
    """Helper function to convert a Python Datetime to a gRPC Timestamp

    Args:
        time_stamp: Python datetime timestamp to convert

    Returns:
        Equivalent gRPC Timestamp
    """
    t = timestamp.Timestamp()
    t.FromDatetime(
        dt=time_stamp if time_stamp is not None else datetime(1970, 1, 1, 0, 0, tzinfo=timezone.utc)
    )
    return t


def timestamp_to_datetime(time_stamp: timestamp.Timestamp) -> Optional[datetime]:
    """Helper function to convert a gRPC Timestamp to a Python Datetime
    Note that datetime has microseconds accuracy instead of nanosecond accuracy for gRPC Timestamp
    Therefore, the conversion may not be lossless.
    Args:
        time_stamp: gRPC Timestamp to convert

    Returns:
        Equivalent Python Datetime
    """
    if time_stamp.seconds == 0 and time_stamp.nanos == 0:
        return
    return time_stamp.ToDatetime(tzinfo=timezone.utc)


def duration_to_timedelta(delta: duration.Duration) -> timedelta:
    """Helper function to convert a gRPC Duration into a Python timedelta
    Note that timedelta has microseconds accuracy instead of nanosecond accuracy for gRPC Duration.
    Therefore, the conversion may not be lossless.
    Args:
        delta: gRPC Duration to convert

    Returns:
        Equivalent Python timedelta
    """
    return delta.ToTimedelta()


def timedelta_to_duration(delta: timedelta) -> duration.Duration:
    """Helper function to convert a gRPC Duration to a Python Datetime

    Args:
        delta: Python timedelta to convert

    Returns:
        Equivalent gRPC Duration
    """
    d = duration.Duration()
    d.FromTimedelta(delta)
    return d


def batched(iterable: Iterable[T], n: int) -> Iterable[List[T]]:
    """
    Batches elements from an iterable into lists of size at most 'n'.

    Args:
        iterable : The input iterable.
        n : The batch size.

    Yields:
        A generator yielding batches of elements from the input iterable.
    """
    it = iter(iterable)

    sentinel = object()
    batch = []
    c = next(it, sentinel)
    while c is not sentinel:
        batch.append(c)
        if len(batch) == n:
            yield batch
            batch = []
        c = next(it, sentinel)
    if len(batch) > 0:
        yield batch
