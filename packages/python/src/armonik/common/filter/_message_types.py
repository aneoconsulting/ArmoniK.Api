from typing import Union, Any, Type, TYPE_CHECKING, TypeVar

from google.protobuf.message import Message

"""
This file allows for better typing and autocompletion support
"""

SimpleOperator = int


class BasicMessage(Message):
    """
    Message type that requires a "field" kwarg
    """

    def __init__(self, *, field, **kwargs):
        self.field = field
        for k, v in kwargs.items():
            setattr(self, k, v)


if TYPE_CHECKING:
    BasicMessageType = Union[Type[BasicMessage], Type[Message]]
else:
    BasicMessageType = BasicMessage


class CombinationMessage(Message):
    """
    Message type for the ands and ors
    """

    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


if TYPE_CHECKING:
    CombinationMessageType = Union[Type[CombinationMessage], Type[Message]]
    DisjunctionType = TypeVar("DisjunctionType", bound=CombinationMessageType)
    ConjunctionType = TypeVar("ConjunctionType", bound=CombinationMessageType)
else:
    CombinationMessageType = CombinationMessage
    DisjunctionType = CombinationMessageType
    ConjunctionType = CombinationMessageType


class InnerMessage(Message):
    """
    Message type for the value and operator
    """

    def __init__(self, *, value: Any, operator: SimpleOperator):
        self.value = value
        self.operator = operator


if TYPE_CHECKING:
    InnerMessageType = Union[Type[InnerMessage], Type[Message]]
else:
    InnerMessageType = InnerMessage
