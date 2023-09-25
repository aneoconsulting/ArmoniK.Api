#!/usr/bin/env python3
from typing import Type

import pytest

from dataclasses import dataclass
from armonik.common.filter import SimpleFilter, FilterConjunction, FilterDisjunction, StringFilter, BooleanFilter, NumberFilter
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


class DummyDisjunction(FilterDisjunction):

    def conjunction_type(self) -> Type["FilterConjunction"]:
        return DummyConjunction

    def message_type(self) -> Type:
        return DummyMessageAnd


class DummyConjunction(FilterConjunction):

    def disjunction_type(self) -> Type["FilterDisjunction"]:
        return DummyDisjunction

    def message_type(self) -> Type:
        return DummyMessage


@pytest.mark.parametrize("filt,inverted", [
    (StringFilter(Field(), DummyConjunction, DummyMessage) == "Test", StringFilter(Field(), DummyConjunction, DummyMessage) != "Test"),
    (StringFilter(Field(), DummyConjunction, DummyMessage).contains("Test"), ~(StringFilter(Field(), DummyConjunction, DummyMessage).contains("Test"))),
    (BooleanFilter(Field(), DummyConjunction, DummyMessage), BooleanFilter(Field(), DummyConjunction, DummyMessage, FilterBoolean, False)),
    (NumberFilter(Field(), DummyConjunction, DummyMessage) > 0, NumberFilter(Field(), DummyConjunction, DummyMessage) <= 0),
    (NumberFilter(Field(), DummyConjunction, DummyMessage) >= 0, NumberFilter(Field(), DummyConjunction, DummyMessage) < 0),
    (NumberFilter(Field(), DummyConjunction, DummyMessage) < 0, NumberFilter(Field(), DummyConjunction, DummyMessage) >= 0),
    (NumberFilter(Field(), DummyConjunction, DummyMessage) <= 0, NumberFilter(Field(), DummyConjunction, DummyMessage) > 0)

])
def test_inversion(filt: SimpleFilter, inverted: SimpleFilter):
    assert filt.operator != inverted.operator or filt.value == (not inverted.value)  # In case of BooleanFilter, the value is inverted, not the operator
    assert (~filt).operator == inverted.operator and (~filt).value == inverted.value
    assert filt.operator == (~(~filt)).operator and filt.value == (~(~filt)).value


@pytest.mark.parametrize("filt", [
    (StringFilter(Field(), DummyConjunction, DummyMessage).startswith("Test")),
    (StringFilter(Field(), DummyConjunction, DummyMessage).endswith("Test")),
    (StringFilter(Field(), DummyConjunction, DummyMessage))  # No op
])
def test_inversion_raises(filt: SimpleFilter):
    with pytest.raises(Exception):
        test = ~filt
        print(test)
