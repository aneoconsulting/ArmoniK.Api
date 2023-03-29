#!/usr/bin/env python3
import pytest

from google.protobuf.timestamp_pb2 import Timestamp
from google.protobuf.duration_pb2 import Duration
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from armonik.common.helpers import datetime_to_timestamp, timestamp_to_datetime, timedelta_to_duration, duration_to_timedelta


@dataclass
class Case:
    seconds: int
    nanos: int

    @property
    def timestamp(self) -> Timestamp:
        return Timestamp(seconds=self.seconds, nanos=self.nanos)

    @property
    def date_time(self) -> datetime:
        return datetime.utcfromtimestamp(self.seconds + self.nanos * 1e-9).replace(tzinfo=timezone.utc)

    @property
    def duration(self) -> Duration:
        return Duration(seconds=self.seconds, nanos=self.nanos)

    @property
    def delta(self) -> timedelta:
        return timedelta(seconds=self.seconds, microseconds=self.nanos / 1000)


test_cases = [
    Case(1234, 1234),
    Case(0, 0),
    Case(12345, 12345),
    Case(100000, 100000)
]


@pytest.mark.parametrize("case", test_cases)
def test_datetime_to_timestamp(case: Case):
    ts = datetime_to_timestamp(case.date_time)
    assert ts.seconds == case.timestamp.seconds and abs(ts.nanos - case.timestamp.nanos) < 1000


@pytest.mark.parametrize("case", test_cases)
def test_timestamp_to_datetime(case: Case):
    ts = timestamp_to_datetime(case.timestamp)
    assert ts.timestamp() == case.date_time.timestamp()


@pytest.mark.parametrize("case", test_cases)
def test_duration_to_timedelta(case: Case):
    ts = duration_to_timedelta(case.duration)
    assert ts.total_seconds() == case.delta.total_seconds()


@pytest.mark.parametrize("case", test_cases)
def test_timedelta_to_duration(case: Case):
    ts = timedelta_to_duration(case.delta)
    assert ts.seconds == case.duration.seconds and abs(ts.nanos - case.duration.nanos) < 1000
