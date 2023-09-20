from typing import List, Any, Type, Union

from protogen.common.filters_common_pb2 import *


class Filter:

    def to_message(self):
        raise NotImplementedError(f"{str(self.__class__.__name__)}.to_message() is not implemented")


class FilterDisjunction(Filter):
    def __init__(self, filters: List["FilterConjunction"]):
        super().__init__()
        self.filters = filters
        if self.conjunction_type() is None:
            raise Exception(f"Conjunction type for {self.__class__} cannot be determined")

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


class FilterConjunction(Filter):
    def __init__(self, filters: List["SimpleFilter"]):
        super().__init__()
        self.filters = filters
        if self.disjunction_type() is None:
            raise Exception(f"Disjunction type for {self.__class__} cannot be determined")

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
        return self.disjunction_type()([self]) | other

    def __add__(self, other: Union["SimpleFilter", "FilterConjunction", "FilterDisjunction"]) -> "FilterDisjunction":
        return self | other

    def __repr__(self):
        return ' and '.join([str(f) for f in self.filters])

    def disjunction_type(self) -> Type["FilterDisjunction"]:
        raise NotImplementedError(f"{str(self.__class__.__name__)}.disjunction_type() is not implemented")


class SimpleFilter(Filter):
    def __init__(self, field: Any, conjunction_type: Type["FilterConjunction"]):
        super().__init__()
        self.field = field
        if not issubclass(conjunction_type, FilterConjunction):
            raise Exception(f"{conjunction_type.__name__} is not a subclass of FilterConjunction")
        self.conjunction_type = conjunction_type

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
        raise NotImplementedError(f"{str(self.__class__.__name__)}.__invert__ does not exist (is this type of filter invertable ?)")

    def __neg__(self) -> "SimpleFilter":
        return ~self

    def __repr__(self) -> str:
        return f"{str(self.field)} filter"


class StringFilter(SimpleFilter):
    def __init__(self, field: Any, conjunction_type: Type["FilterConjunction"], value=None, operator=None):
        super().__init__(field, conjunction_type)
        self.raw = FilterString()
        if value is not None:
            self.raw.value = value
        if operator is not None:
            self.raw.operator = operator

    def contains(self, value: str) -> "StringFilter":
        return StringFilter(self.field, self.conjunction_type, value, FILTER_STRING_OPERATOR_CONTAINS)

    def __eq__(self, value: str) -> "StringFilter":
        return StringFilter(self.field, self.conjunction_type, value, FILTER_STRING_OPERATOR_EQUAL)

    def __ne__(self, value: str) -> "StringFilter":
        return StringFilter(self.field, self.conjunction_type, value, FILTER_STRING_OPERATOR_NOT_EQUAL)

    def __invert__(self) -> "StringFilter":
        if self.raw.operator == FILTER_STRING_OPERATOR_EQUAL:
            return StringFilter(self.field, self.conjunction_type, self.raw.value, FILTER_STRING_OPERATOR_NOT_EQUAL)
        elif self.raw.operator == FILTER_STRING_OPERATOR_NOT_EQUAL:
            return StringFilter(self.field, self.conjunction_type, self.raw.value, FILTER_STRING_OPERATOR_EQUAL)
        elif self.raw.operator == FILTER_STRING_OPERATOR_CONTAINS:
            return StringFilter(self.field, self.conjunction_type, self.raw.value, FILTER_STRING_OPERATOR_NOT_CONTAINS)
        elif self.raw.operator == FILTER_STRING_OPERATOR_NOT_CONTAINS:
            return StringFilter(self.field, self.conjunction_type, self.raw.value, FILTER_STRING_OPERATOR_CONTAINS)
        else:
            raise Exception(f"String filter operator {str(self.raw.operator)} for field {str(self.field)} has no inverted equivalent")

    def startswith(self, value: str) -> "StringFilter":
        return StringFilter(self.field, self.conjunction_type, value, FILTER_STRING_OPERATOR_STARTS_WITH)

    def endswith(self, value: str) -> "StringFilter":
        return StringFilter(self.field, self.conjunction_type, value, FILTER_STRING_OPERATOR_ENDS_WITH)

    def to_message(self) -> FilterString:
        return self.raw

    def __repr__(self) -> str:
        return f"{str(self.field)} {str(self.raw.operator)} {str(self.raw.value)}"


