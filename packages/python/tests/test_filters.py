#!/usr/bin/env python3
import itertools
from typing import Dict, cast, Union

import pytest

from dataclasses import dataclass

from armonik.common import Task
from armonik.common.filter import (
    Filter,
    StringFilter,
    BooleanFilter,
    NumberFilter,
    DisjunctionType,
    ConjunctionType,
    BasicMessageType,
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

def test_descriptor():
    assert isinstance(Task.id == "xxx", Filter)
    Task().id


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
