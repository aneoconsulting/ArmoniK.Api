#!/usr/bin/env python3
import itertools
from typing import Dict, cast, Union

import pytest

from dataclasses import dataclass

from armonik.common import Task, Partition
from armonik.common.filter import (
    Filter,
    StringFilter,
    BooleanFilter,
    NumberFilter,
    DisjunctionType,
    ConjunctionType,
    BasicMessageType,
    TaskOptionFilter,
    FilterError,
)
from armonik.protogen.common.filters_common_pb2 import FilterBoolean
from google.protobuf.message import Message


@dataclass
class DummyMessage(DisjunctionType):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._disjunction = []

    def __getattr__(self, item):
        if item == "or":
            return self._disjunction
        raise AttributeError()


@dataclass
class DummyMessageAnd(ConjunctionType):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._conjunction = []

    def __getattr__(self, item):
        if item == "and":
            return self._conjunction
        raise AttributeError()


@dataclass
class Field(Message):
    name: str = ""


@dataclass
class BaseMessage(BasicMessageType):
    def __init__(self, *, field, **kwargs):
        super().__init__(field=field, **kwargs)


@pytest.mark.parametrize(
    "filt,inverted",
    [
        (
            StringFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) == "Test",
            StringFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) != "Test",
        ),
        (
            StringFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage).contains("Test"),
            ~(StringFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage).contains("Test")),
        ),
        (
            BooleanFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage),
            BooleanFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage, value=False),
        ),
        (
            NumberFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) > 0,
            NumberFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) <= 0,
        ),
        (
            NumberFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) >= 0,
            NumberFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) < 0,
        ),
        (
            NumberFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) < 0,
            NumberFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) >= 0,
        ),
        (
            NumberFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) <= 0,
            NumberFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage) > 0,
        ),
    ],
)
def test_inversion(filt: Filter, inverted: Filter):
    assert filt.operator != inverted.operator or filt.value == (
        not inverted.value
    )  # In case of BooleanFilter, the value is inverted, not the operator
    assert (~filt).operator == inverted.operator and (~filt).value == inverted.value
    assert filt.operator == (~(~filt)).operator and filt.value == (~(~filt)).value


@pytest.mark.parametrize(
    "filt",
    [
        (StringFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage).startswith("Test")),
        (StringFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage).endswith("Test")),
        (StringFilter(Field(), DummyMessage, DummyMessageAnd, BaseMessage)),  # No op
    ],
)
def test_inversion_raises(filt: Filter):
    with pytest.raises(Exception):
        test = ~filt
        print(test)


def test_descriptor():
    task = Task()
    assert task.id is None
    task.id = "xxx"
    assert isinstance(task.id, str)
    assert isinstance(Task.id, Filter)
    assert isinstance(Task.id == "xxx", Filter)


def test_extra_operator():
    with pytest.raises(FilterError):
        try:
            print((Task.id == "x") == "y")
        except FilterError as e:
            print(e)
            raise

    with pytest.raises(FilterError):
        try:
            print(((Task.id == "x") & (Task.session_id == "y")) == "y")
        except FilterError as e:
            print(e)
            raise


def test_extra_bool_compare():
    def check_eq(f1: Filter, f2: Filter) -> bool:
        return f1.operator == f2.operator and f1.value == f2.value

    task_filt = cast(Filter, Task.id == "x")
    task_filt_not = cast(Filter, Task.id != "x")

    assert isinstance(task_filt, Filter)
    assert isinstance(task_filt_not, Filter)

    task_filt_eq_true = cast(Filter, task_filt == True)  # noqa: E712
    task_filt_eq_false = cast(Filter, task_filt == False)  # noqa: E712
    task_filt_ne_true = cast(Filter, task_filt != True)  # noqa: E712
    task_filt_ne_false = cast(Filter, task_filt != False)  # noqa: E712
    assert check_eq(task_filt_eq_true, task_filt)
    assert not check_eq(task_filt_eq_true, task_filt_not)
    assert not check_eq(task_filt_eq_false, task_filt)
    assert check_eq(task_filt_eq_false, task_filt_not)
    assert not check_eq(task_filt_ne_true, task_filt)
    assert check_eq(task_filt_ne_true, task_filt_not)
    assert check_eq(task_filt_ne_false, task_filt)
    assert not check_eq(task_filt_ne_false, task_filt_not)


def test_task_option_descriptor():
    assert isinstance(Task.options, TaskOptionFilter)
    assert isinstance(Task.options.priority, Filter)
    with pytest.raises(FilterError):
        try:
            print(Task.options == "test")
        except FilterError as e:
            print(e)
            raise
    with pytest.raises(FilterError):
        try:
            print(Task.options.contains("test"))
        except FilterError as e:
            print(e)
            raise


def test_unavailable_field():
    with pytest.raises(FilterError):
        print(Partition.pod_configuration == {})


def apply_filter(state: Dict[str, bool], message: Union[Message, Filter]) -> bool:
    if isinstance(message, Filter):
        return apply_filter(state, message.to_message())
    if hasattr(message, "or"):
        return any(apply_filter(state, m) for m in getattr(message, "or"))
    if hasattr(message, "and"):
        return all(apply_filter(state, m) for m in getattr(message, "and"))
    vp = cast(FilterBoolean, getattr(message, "filter_boolean"))
    field = cast(Field, getattr(message, "field"))
    return vp.value == state[field.name]


def get_bool_filter(name: str) -> BooleanFilter:
    return BooleanFilter(Field(name), DummyMessage, DummyMessageAnd, BaseMessage)


def test_combine():
    a = get_bool_filter("a")
    b = get_bool_filter("b")
    c = get_bool_filter("c")
    d = get_bool_filter("d")
    e = get_bool_filter("e")
    for _a, _b, _c, _d, _e in itertools.product([True, False], repeat=5):
        t = {"a": _a, "b": _b, "c": _c, "d": _d, "e": _e}
        assert apply_filter(t, a) == _a
        assert apply_filter(t, (~a)) == (not _a)
        assert apply_filter(t, (a | b)) == (_a or _b)
        assert apply_filter(t, ((~a) | b)) == ((not _a) or _b)
        assert apply_filter(t, (a | (~b))) == (_a or not _b)
        assert apply_filter(t, ((~a) | (~b))) == (not _a or not _b)
        assert apply_filter(t, ((~a) & b)) == ((not _a) and _b)
        assert apply_filter(t, (~(a | b))) == (not (_a or _b))
        assert apply_filter(t, (~(a & b))) == (not (_a and _b))
        assert apply_filter(t, (c & (a | b))) == (_c and (_a or _b))
        assert apply_filter(t, (c | (a & b))) == (_c or (_a and _b))
        assert apply_filter(t, (a & (b | c)) & (d | e)) == (_a and (_b or _c) and (_d or _e))
        assert apply_filter(t, ~((a & (b | c)) & (d | e))) == (
            not (_a and (_b or _c) and (_d or _e))
        )
