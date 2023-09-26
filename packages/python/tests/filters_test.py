#!/usr/bin/env python3
from typing import Type

import pytest

from dataclasses import dataclass
from armonik.common.filter import Filter, StringFilter, BooleanFilter, NumberFilter
from armonik.protogen.common.filters_common_pb2 import FilterBoolean
from google.protobuf.message import Message


@dataclass
class DummyMessage(Message):
    pass


@dataclass
class DummyMessageAnd(Message):
    pass


@dataclass
class Field(Message):
    pass


@pytest.mark.parametrize("filt,inverted", [
    (StringFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) == "Test", StringFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) != "Test"),
    (StringFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage).contains("Test"), ~(StringFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage).contains("Test"))),
    (BooleanFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage), BooleanFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage, FilterBoolean, None, False)),
    (NumberFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) > 0, NumberFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) <= 0),
    (NumberFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) >= 0, NumberFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) < 0),
    (NumberFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) < 0, NumberFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) >= 0),
    (NumberFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) <= 0, NumberFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage) > 0)

])
def test_inversion(filt: Filter, inverted: Filter):
    assert filt.operator != inverted.operator or filt.value == (not inverted.value)  # In case of BooleanFilter, the value is inverted, not the operator
    assert (~filt).operator == inverted.operator and (~filt).value == inverted.value
    assert filt.operator == (~(~filt)).operator and filt.value == (~(~filt)).value


@pytest.mark.parametrize("filt", [
    (StringFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage).startswith("Test")),
    (StringFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage).endswith("Test")),
    (StringFilter(Field(), DummyMessage, DummyMessageAnd, DummyMessage))  # No op
])
def test_inversion_raises(filt: Filter):
    with pytest.raises(Exception):
        test = ~filt
        print(test)
