#!/usr/bin/env python3
import pytest

from google.protobuf.timestamp_pb2 import Timestamp
from google.protobuf.duration_pb2 import Duration
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from armonik.common.helpers import (
    datetime_to_timestamp,
    timestamp_to_datetime,
    timedelta_to_duration,
    duration_to_timedelta,
    batched,
)

from typing import Iterable, List


@dataclass
class Case:
    seconds: int
    nanos: int

    @property
    def timestamp(self) -> Timestamp:
        return Timestamp(seconds=self.seconds, nanos=self.nanos)

    @property
    def date_time(self) -> datetime:
        if self.seconds == 0 and self.nanos == 0:
            return
        return datetime.utcfromtimestamp(self.seconds + self.nanos * 1e-9).replace(
            tzinfo=timezone.utc
        )

    @property
    def duration(self) -> Duration:
        return Duration(seconds=self.seconds, nanos=self.nanos)

    @property
    def delta(self) -> timedelta:
        return timedelta(seconds=self.seconds, microseconds=self.nanos / 1000)


test_cases = [Case(1234, 1234), Case(0, 0), Case(12345, 12345), Case(100000, 100000)]


@pytest.mark.parametrize("case", test_cases)
def test_datetime_to_timestamp(case: Case):
    ts = datetime_to_timestamp(case.date_time)
    assert ts.seconds == case.timestamp.seconds and abs(ts.nanos - case.timestamp.nanos) < 1000


@pytest.mark.parametrize("case", test_cases)
def test_timestamp_to_datetime(case: Case):
    dt = timestamp_to_datetime(case.timestamp)
    assert dt == case.date_time


@pytest.mark.parametrize("case", test_cases)
def test_duration_to_timedelta(case: Case):
    ts = duration_to_timedelta(case.duration)
    assert ts.total_seconds() == case.delta.total_seconds()


@pytest.mark.parametrize("case", test_cases)
def test_timedelta_to_duration(case: Case):
    ts = timedelta_to_duration(case.delta)
    assert ts.seconds == case.duration.seconds and abs(ts.nanos - case.duration.nanos) < 1000


@pytest.mark.parametrize(
    ["iterable", "batch_size", "iterations"],
    [
        ([1, 2, 3], 3, [[1, 2, 3]]),
        ([1, 2, 3], 5, [[1, 2, 3]]),
        ([1, 2, 3], 2, [[1, 2], [3]]),
        (
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            3,
            [[1, 2, 3], [4, 5, 6], [7, 8, 9], [10, 11]],
        ),
    ],
)
def test_batched(iterable: Iterable, batch_size: int, iterations: List[Iterable]):
    for index, batch in enumerate(batched(iterable, batch_size)):
        assert batch == iterations[index]
