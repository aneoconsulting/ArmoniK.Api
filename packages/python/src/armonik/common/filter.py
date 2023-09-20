from typing import List, Any, Type, Union
from google.protobuf.message import Message
import google.protobuf.timestamp_pb2 as timestamp
from protogen.common.filters_common_pb2 import *
from datetime import datetime
from ..common.helpers import datetime_to_timestamp


class Filter:
    def message_type(self) -> Type:
        raise NotImplementedError(f"{str(self.__class__.__name__)}.message_type() is not implemented")

    def to_message(self):
        raise NotImplementedError(f"{str(self.__class__.__name__)}.to_message() is not implemented")

    def __bool__(self):
        raise Exception("Filter cannot be converted to bool. Are you trying to use 'and', 'or', 'not' or 'in' instead of '&', '|', '~' or '.contains' ?")

    def to_conjunction(self) -> "FilterConjunction":
        raise NotImplementedError(f"{str(self.__class__.__name__)}.to_conjunction() is not implemented")

    def to_disjunction(self) -> "FilterDisjunction":
        raise NotImplementedError(f"{str(self.__class__.__name__)}.to_disjunction() is not implemented")


class FilterDisjunction(Filter):
    def __init__(self, filters: List["FilterConjunction"]):
        super().__init__()
        self.filters = filters
        _ = self.conjunction_type()
        _ = self.message_type()

    def __or__(self, other: Union["SimpleFilter", "FilterConjunction", "FilterDisjunction"]) -> "FilterDisjunction":
        if isinstance(other, SimpleFilter):
            if other.conjunction_type != self.conjunction_type():
                raise Exception(f"Invalid type {type(other).__name__} ({str(other.field)}) for 'or' operand of {self.__class__.__name__} ({str(self)}) : Conjunction types are different")
            return self.__class__(self.filters + [self.conjunction_type()([other])])
        elif isinstance(other, self.conjunction_type()):
            return self.__class__(self.filters + [other])
        elif isinstance(other, self.__class__):
            return self.__class__(self.filters + other.filters)
        raise Exception(f"Invalid type {type(other).__name__} for 'or' operand of {self.__class__.__name__}")

    def __add__(self, other: Union["SimpleFilter", "FilterConjunction", "FilterDisjunction"]) -> "FilterDisjunction":
        return self | other

    def __repr__(self):
        return "( " + ' ) or ( '.join([str(f) for f in self.filters]) + ' )'

    def conjunction_type(self) -> Type["FilterConjunction"]:
        raise NotImplementedError(f"{str(self.__class__.__name__)}.conjunction_type() is not implemented")

    def to_message(self):
        raw = self.message_type()()
        setattr(raw, "or", [f.to_message() for f in self.filters])
        return raw

    def to_conjunction(self) -> "FilterConjunction":
        raise Exception("Cannot transform a disjunction into a conjunction")

    def to_disjunction(self) -> "FilterDisjunction":
        return self


class FilterConjunction(Filter):
    def __init__(self, filters: List["SimpleFilter"]):
        super().__init__()
        self.filters = filters
        _ = self.disjunction_type()
        _ = self.message_type()

    def __and__(self, other: Union["SimpleFilter", "FilterConjunction"]) -> "FilterConjunction":
        if isinstance(other, SimpleFilter):
            if other.conjunction_type != self.__class__:
                raise Exception(f"Invalid type {type(other).__name__} ({str(other.field)}) for 'and' operand of {self.__class__.__name__} ({str(self)}) : Conjunction types are different")
            return self.__class__(self.filters + [other])
        elif isinstance(other, self.__class__):
            return self.__class__(self.filters + other.filters)
        raise Exception(f"Invalid type {type(other).__name__} for 'and' operand of {self.__class__.__name__}")

    def __mul__(self, other: Union["SimpleFilter", "FilterConjunction"]) -> "FilterConjunction":
        return self & other

    def __or__(self, other: Union["SimpleFilter", "FilterConjunction", "FilterDisjunction"]) -> "FilterDisjunction":
        return self.to_disjunction() | other

    def __add__(self, other: Union["SimpleFilter", "FilterConjunction", "FilterDisjunction"]) -> "FilterDisjunction":
        return self | other

    def __repr__(self):
        return ' and '.join([str(f) for f in self.filters])

    def disjunction_type(self) -> Type["FilterDisjunction"]:
        raise NotImplementedError(f"{str(self.__class__.__name__)}.disjunction_type() is not implemented")

    def to_message(self):
        raw = self.message_type()()
        setattr(raw, "and", [f.to_message() for f in self.filters])
        return raw

    def to_conjunction(self) -> "FilterConjunction":
        return self

    def to_disjunction(self) -> "FilterDisjunction":
        return self.disjunction_type()([self])


class SimpleFilter(Filter):
    def __init__(self, field: Message, conjunction_type: Type["FilterConjunction"], message_type: Type[Message], inner_message_type: Type[Message], value=None, operator=None):
        super().__init__()
        self.field = field
        self.message_type = message_type
        if not issubclass(conjunction_type, FilterConjunction):
            raise Exception(f"{conjunction_type.__name__} is not a subclass of FilterConjunction")
        self.conjunction_type = conjunction_type
        self.inner_message_type = inner_message_type
        self.raw = inner_message_type()
        if value is not None:
            self.raw.value = value
        if operator is not None:
            self.raw.operator = operator

    def __and__(self, other: Union["SimpleFilter", "FilterConjunction"]) -> "FilterConjunction":
        if isinstance(other, SimpleFilter):
            if other.conjunction_type != self.conjunction_type:
                raise Exception(f"Invalid type {type(other).__name__} ({str(other.field)}) for 'and' operand of {self.__class__.__name__} ({str(self.field)}) : Conjunction types are different")
            return self.conjunction_type([self, other])
        elif isinstance(other, self.conjunction_type):
            return other & self
        raise Exception(f"Invalid type {type(other).__name__} for 'and' operand of {self.__class__.__name__}")

    def __mul__(self, other: Union["SimpleFilter", "FilterConjunction"]) -> "FilterConjunction":
        return self & other

    def __or__(self, other: Union["SimpleFilter", "FilterConjunction", "FilterDisjunction"]) -> "FilterDisjunction":
        return self.conjunction_type([self]) | other

    def __add__(self, other: Union["SimpleFilter", "FilterConjunction", "FilterDisjunction"]) -> "FilterDisjunction":
        return self | other

    def __invert__(self):
        raise NotImplementedError(f"{str(self.__class__.__name__)}.__invert__ does not exist (is this type of filter invertible ?)")

    def __neg__(self) -> "SimpleFilter":
        return ~self

    def __repr__(self) -> str:
        return f"{str(self.field)} filter"

    def to_conjunction(self) -> "FilterConjunction":
        return self.conjunction_type([self])

    def to_disjunction(self) -> "FilterDisjunction":
        return self.to_conjunction().to_disjunction()

    def check(self, value: Any, operator: Any) -> "SimpleFilter":
        return self.__class__(self.field, self.conjunction_type, self.message_type, self.inner_message_type, value, operator)


class StringFilter(SimpleFilter):
    def __init__(self, field: Message, conjunction_type: Type["FilterConjunction"], message_type: Type[Message], inner_message_type: Type[Message] = FilterString, value=None, operator=None):
        super().__init__(field, conjunction_type, message_type, inner_message_type, value, operator)

    def contains(self, value: str) -> "StringFilter":
        return self.check(value, FILTER_STRING_OPERATOR_CONTAINS)

    def __eq__(self, value: str) -> "StringFilter":
        return self.check(value, FILTER_STRING_OPERATOR_EQUAL)

    def __ne__(self, value: str) -> "StringFilter":
        return self.check(value, FILTER_STRING_OPERATOR_NOT_EQUAL)

    def __invert__(self) -> "StringFilter":
        if self.raw.operator == FILTER_STRING_OPERATOR_EQUAL:
            return self.check(self.raw.value, FILTER_STRING_OPERATOR_NOT_EQUAL)
        elif self.raw.operator == FILTER_STRING_OPERATOR_NOT_EQUAL:
            return self.check(self.raw.value, FILTER_STRING_OPERATOR_EQUAL)
        elif self.raw.operator == FILTER_STRING_OPERATOR_CONTAINS:
            return self.check(self.raw.value, FILTER_STRING_OPERATOR_NOT_CONTAINS)
        elif self.raw.operator == FILTER_STRING_OPERATOR_NOT_CONTAINS:
            return self.check(self.raw.value, FILTER_STRING_OPERATOR_CONTAINS)
        else:
            raise Exception(f"String filter operator {str(self.raw.operator)} for field {str(self.field)} has no inverted equivalent")

    def startswith(self, value: str) -> "StringFilter":
        return self.check(value, FILTER_STRING_OPERATOR_STARTS_WITH)

    def endswith(self, value: str) -> "StringFilter":
        return self.check(value, FILTER_STRING_OPERATOR_ENDS_WITH)

    def to_message(self):
        message = self.message_type()
        message.field = self.field
        message.filter_string = self.raw
        return message

    def __repr__(self) -> str:
        return f"{str(self.field)} {str(self.raw.operator)} \"{str(self.raw.value)}\""


class StatusFilter(SimpleFilter):
    def __init__(self, field: Message, conjunction_type: Type["FilterConjunction"], message_type: Type[Message], filter_status_type: Type[Message], value=None, operator=None):
        super().__init__(field, conjunction_type, message_type, filter_status_type, value, operator)

    def __eq__(self, value) -> "StatusFilter":
        return self.check(value, FILTER_STATUS_OPERATOR_EQUAL)

    def __ne__(self, value) -> "StatusFilter":
        return self.check(value, FILTER_STATUS_OPERATOR_NOT_EQUAL)

    def __invert__(self) -> "StatusFilter":
        if self.raw.operator == FILTER_STATUS_OPERATOR_EQUAL:
            return self != self.raw.value
        elif self.raw.operator == FILTER_STATUS_OPERATOR_NOT_EQUAL:
            return self == self.raw.value
        else:
            raise Exception(f"Status filter operator {str(self.raw.operator)} for field {str(self.field)} has no inverted equivalent")

    def to_message(self):
        message = self.message_type()
        message.field = self.field
        message.filter_status = self.raw
        return message

class DateFilter(SimpleFilter):
    def __init__(self, field: Message, conjunction_type: Type["FilterConjunction"], message_type: Type[Message], inner_message_type: Type[Message] = FilterDate, value=None, operator=None):
        super().__init__(field, conjunction_type, message_type, inner_message_type, value, operator)

    def normalize_value(self, value) -> timestamp.Timestamp:
        if isinstance(value, timestamp.Timestamp):
            return value
        if isinstance(value, datetime):
            return datetime_to_timestamp(value)

    def __eq__(self, value) -> "DateFilter":
        return self.check(self.normalize_value(value), FILTER_DATE_OPERATOR_EQUAL)

    def __ne__(self, value) -> "DateFilter":
        return self.check(self.normalize_value(value), FILTER_DATE_OPERATOR_NOT_EQUAL)


