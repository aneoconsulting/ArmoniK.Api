from __future__ import annotations
from abc import abstractmethod
from typing import List, Any, Type, Optional, Dict
from google.protobuf.message import Message
import google.protobuf.timestamp_pb2 as timestamp
import google.protobuf.duration_pb2 as duration
from ..protogen.common.filters_common_pb2 import *
import json


class Filter:
    """
    Filter for use with ArmoniK

    Attributes:
        eq_: equality raw Api operator
        ne_: inequality raw Api operator
        lt_: less than raw Api operator
        le_: less or equal raw Api operator
        gt_: greater than raw Api operator
        ge_: greater or equal raw Api operator
        contains_: contains raw Api operator
        notcontains_: not contains raw Api operator
        value_type_: expected type for the value to test against in this filter

        field: field of the filter if it's a simple filter
        message_type: Api message type of the filter
        inner_message_type: Api message type of the inner filter (with value and operator)
        conjunction_type: Type of the conjunction for this filter
        value: value to test against in this filter if it's a simple filter
        operator: operator to apply for this filter if it's a simple filter
    """
    eq_ = None
    ne_ = None
    lt_ = None
    le_ = None
    gt_ = None
    ge_ = None
    contains_ = None
    notcontains_ = None
    value_type_ = None

    def __init__(self, field: Optional[Message], disjunction_message_type: Type[Message], conjunction_message_type: Type[Message], message_type: Type[Message], inner_message_type: Optional[Type[Message]], filters: Optional[List[List["Filter"]]] = None, value=None, operator=None):
        self._filters: List[List["Filter"]] = [[]] if filters is None else filters
        self.field = field
        self.message_type = message_type
        self.conjunction_type = conjunction_message_type
        self.disjunction_type = disjunction_message_type
        self.inner_message_type = inner_message_type
        self.value = value
        self.operator = operator

    def is_true_conjunction(self) -> bool:
        """
        Tests whether the filter is a conjunction (logical and)
        Note : This will only output true if it's an actual conjunction with multiple filters and no disjunction
        """
        return self.message_type == self.conjunction_type or (len(self._filters) == 1 and len(self._filters[0]) > 1)

    def is_true_disjunction(self) -> bool:
        """
        Tests whether the filter is a disjunction (logical or)
        Note : This will only output true if it's an actual disjunction with multiple filters
        """
        return len(self._filters) > 1

    def to_disjunction(self) -> Filter:
        """
        Converts the filter into a disjunction

        """
        if self.is_true_disjunction():
            return self
        if self.is_true_conjunction():
            return Filter(None, self.disjunction_type, self.conjunction_type, self.disjunction_type, None, self._filters)
        return Filter(None, self.disjunction_type, self.conjunction_type, self.disjunction_type, None, [[self]])

    def __and__(self, other: "Filter") -> "Filter":
        if not isinstance(other, Filter):
            msg = f"Cannot create a conjunction between Filter and {other.__class__.__name__}"
            raise Exception(msg)
        if self.is_true_disjunction() or other.is_true_disjunction():
            raise Exception("Cannot make a conjunction of disjunctions")
        if self.conjunction_type != other.conjunction_type:
            raise Exception("Conjunction types are different")
        return Filter(None, self.disjunction_type, self.conjunction_type, self.conjunction_type, None, [self.to_disjunction()._filters[0] + other.to_disjunction()._filters[0]])

    def __mul__(self, other: Filter) -> "Filter":
        return self & other

    def __or__(self, other: "Filter") -> "Filter":
        if not isinstance(other, Filter):
            msg = f"Cannot create a conjunction between Filter and {other.__class__.__name__}"
            raise Exception(msg)
        if self.disjunction_type != other.disjunction_type:
            raise Exception("Disjunction types are different")
        return Filter(None, self.disjunction_type, self.conjunction_type, self.disjunction_type, None, self.to_disjunction()._filters + other.to_disjunction()._filters)

    def __add__(self, other: "Filter") -> "Filter":
        return self | other

    def __eq__(self, value) -> Filter:
        return self._check(value, self.__class__.eq_, "==")

    def __ne__(self, value) -> Filter:
        return self._check(value, self.__class__.ne_, "!=")

    def __lt__(self, value) -> Filter:
        return self._check(value, self.__class__.lt_, "<")

    def __le__(self, value) -> Filter:
        return self._check(value, self.__class__.le_, "<=")

    def __gt__(self, value) -> Filter:
        return self._check(value, self.__class__.gt_, ">")

    def __ge__(self, value) -> Filter:
        return self._check(value, self.__class__.ge_, ">=")

    def contains(self, value) -> Filter:
        return self._check(value, self.__class__.contains_, "contains")

    def __invert__(self) -> Filter:
        """
        Inverts the test

        Returns:
            Filter with the test being inverted
        """
        if self.operator is None:
            if self.is_true_conjunction() or self.is_true_disjunction():
                raise Exception("Cannot invert conjunctions or disjunctions")
            msg = f"Cannot invert None operator in class {self.__class__.__name__} for field {str(self.field)}"
            raise Exception(msg)
        if self.operator == self.__class__.eq_:
            return self.__ne__(self.value)
        if self.operator == self.__class__.ne_:
            return self.__eq__(self.value)
        if self.operator == self.__class__.lt_:
            return self.__ge__(self.value)
        if self.operator == self.__class__.le_:
            return self.__gt__(self.value)
        if self.operator == self.__class__.gt_:
            return self.__le__(self.value)
        if self.operator == self.__class__.ge_:
            return self.__lt__(self.value)
        if self.operator == self.__class__.contains_:
            return self._check(self.value, self.__class__.notcontains_, "not_contains")
        if self.operator == self.__class__.notcontains_:
            return self.contains(self.value)
        msg = f"{self.__class__.__name__} operator {str(self.operator)} for field {str(self.field)} has no inverted equivalent"
        raise Exception(msg)

    def __neg__(self) -> "Filter":
        return ~self

    def to_dict(self) -> Dict:
        rep = {}
        if self.is_true_disjunction():
            rep["or"] = [{"and": [f.to_dict() for f in conj]} for conj in self._filters]
            return rep
        if self.is_true_conjunction():
            rep["and"] = [f.to_dict() for f in self._filters[0]]
            return rep
        if len(self._filters) == 1 and len(self._filters[0]) == 1:
            return self._filters[0][0].to_dict()
        return {"field": str(self.field), "value": str(self.value), "operator": str(self.operator)}

    def __str__(self) -> str:
        return json.dumps(self.to_dict())

    def _verify_value(self, value):
        """
        Checks if the value is of the expected type
        Args:
            value: Value to test

        Raises:
            Exception if value is not of the expected type

        """
        if self.__class__.value_type_ is None or isinstance(value, self.__class__.value_type_):
            return
        msg = f"Expected value type {str(self.__class__.value_type_)} for field {str(self.field)}, got {str(type(value))} instead"
        raise Exception(msg)

    def _check(self, value: Any, operator: Any, operator_str: str = "") -> "Filter":
        """
        Internal function to create a new filter from the current filter with a different value and/or operator
        Args:
            value: Value of the new filter
            operator: Operator of the new filter
            operator_str: Optional string for error message clarification

        Returns:
            new filter with the given value/operator

        Raises:
            NotImplementedError if the given operator is not available for the given class
        """
        if self.is_true_conjunction() or self.is_true_disjunction():
            raise Exception("Cannot apply operator to a disjunction or a conjunction")
        self._verify_value(value)
        if operator is None:
            msg = f"Operator {operator_str} is not available for {self.__class__.__name__}"
            raise NotImplementedError(msg)
        return self.__class__(self.field, self.disjunction_type, self.conjunction_type, self.message_type, self.inner_message_type, self._filters, value, operator)

    @abstractmethod
    def to_basic_message(self) -> Message:
        pass

    def to_message(self) -> Message:
        def to_conjunction_message(conj: List[Filter]) -> Message:
            conj_raw = self.conjunction_type()
            getattr(conj_raw, "and").extend([f.to_basic_message() for f in conj])
            return conj_raw

        if self.message_type == self.disjunction_type:
            raw = self.to_disjunction().disjunction_type()
            getattr(raw, "or").extend([to_conjunction_message(conj) for conj in self._filters])
            return raw
        if self.message_type == self.conjunction_type:
            return to_conjunction_message(self.to_disjunction()._filters[0])
        return self.to_basic_message()


class StringFilter(Filter):
    """
    Filter for string comparisons
    """
    eq_ = FILTER_STRING_OPERATOR_EQUAL
    ne_ = FILTER_STRING_OPERATOR_NOT_EQUAL
    contains_ = FILTER_STRING_OPERATOR_CONTAINS
    notcontains_ = FILTER_STRING_OPERATOR_NOT_CONTAINS
    value_type_ = str

    def __init__(self, field: Optional[Message], disjunction_message_type: Type[Message], conjunction_message_type: Type[Message], message_type: Type[Message], inner_message_type: Optional[Type[Message]] = FilterString, filters: Optional[List[List["Filter"]]] = None, value=None, operator=None):
        super().__init__(field, disjunction_message_type, conjunction_message_type, message_type, inner_message_type, filters, value, operator)

    def startswith(self, value: str) -> "StringFilter":
        return self._check(value, FILTER_STRING_OPERATOR_STARTS_WITH, "startswith")

    def endswith(self, value: str) -> "StringFilter":
        return self._check(value, FILTER_STRING_OPERATOR_ENDS_WITH, "endswith")

    def to_basic_message(self) -> Message:
        return self.message_type(field=self.field, filter_string=self.inner_message_type(value=self.value, operator=self.operator))

    def __repr__(self) -> str:
        return f"{str(self.field)} {str(self.operator)} \"{str(self.value)}\""


class StatusFilter(Filter):
    """
    Filter for status comparison
    """
    eq_ = FILTER_STATUS_OPERATOR_EQUAL
    ne_ = FILTER_STATUS_OPERATOR_NOT_EQUAL

    def __init__(self, field: Optional[Message], disjunction_message_type: Type[Message], conjunction_message_type: Type[Message], message_type: Type[Message], inner_message_type: Type[Message], filters: Optional[List[List["Filter"]]] = None, value=None, operator=None):
        super().__init__(field, disjunction_message_type, conjunction_message_type, message_type, inner_message_type, filters, value, operator)

    def to_basic_message(self) -> Message:
        return self.message_type(field=self.field, filter_status=self.inner_message_type(value=self.value, operator=self.operator))


class DateFilter(Filter):
    """
    Filter for timestamp comparison
    """
    eq_ = FILTER_DATE_OPERATOR_EQUAL
    ne_ = FILTER_DATE_OPERATOR_NOT_EQUAL
    lt_ = FILTER_DATE_OPERATOR_BEFORE
    le_ = FILTER_DATE_OPERATOR_BEFORE_OR_EQUAL
    gt_ = FILTER_DATE_OPERATOR_AFTER
    ge_ = FILTER_DATE_OPERATOR_AFTER_OR_EQUAL
    value_type = timestamp.Timestamp

    def __init__(self, field: Optional[Message], disjunction_message_type: Type[Message], conjunction_message_type: Type[Message], message_type: Type[Message], inner_message_type: Optional[Type[Message]] = FilterDate, filters: Optional[List[List["Filter"]]] = None, value=None, operator=None):
        super().__init__(field, disjunction_message_type, conjunction_message_type, message_type, inner_message_type, filters, value, operator)

    def to_basic_message(self) -> Message:
        return self.message_type(field=self.field, filter_date=self.inner_message_type(value=self.value, operator=self.operator))


class NumberFilter(Filter):
    """
    Filter for int comparison
    """
    eq_ = FILTER_NUMBER_OPERATOR_EQUAL
    ne_ = FILTER_NUMBER_OPERATOR_NOT_EQUAL
    lt_ = FILTER_NUMBER_OPERATOR_LESS_THAN
    le_ = FILTER_NUMBER_OPERATOR_LESS_THAN_OR_EQUAL
    gt_ = FILTER_NUMBER_OPERATOR_GREATER_THAN
    ge_ = FILTER_NUMBER_OPERATOR_GREATER_THAN_OR_EQUAL
    value_type_ = int

    def __init__(self, field: Optional[Message], disjunction_message_type: Type[Message], conjunction_message_type: Type[Message], message_type: Type[Message], inner_message_type: Optional[Type[Message]] = FilterNumber, filters: Optional[List[List["Filter"]]] = None, value=None, operator=None):
        super().__init__(field, disjunction_message_type, conjunction_message_type, message_type, inner_message_type, filters, value, operator)

    def to_basic_message(self) -> Message:
        return self.message_type(field=self.field, filter_number=self.inner_message_type(value=self.value, operator=self.operator))


class BooleanFilter(Filter):
    """
    Filter for boolean comparison
    """
    eq_ = FILTER_BOOLEAN_OPERATOR_IS
    value_type_ = bool

    def __init__(self, field: Optional[Message], disjunction_message_type: Type[Message], conjunction_message_type: Type[Message], message_type: Type[Message], inner_message_type: Optional[Type[Message]] = FilterBoolean, filters: Optional[List[List["Filter"]]] = None, value=True, operator=FILTER_BOOLEAN_OPERATOR_IS):
        super().__init__(field, disjunction_message_type, conjunction_message_type, message_type, inner_message_type, filters, value, operator)

    def __ne__(self, value: bool) -> "BooleanFilter":
        return self.__eq__(not value)

    def __invert__(self) -> "BooleanFilter":
        return self.__eq__(not self.value)

    def to_basic_message(self) -> Message:
        return self.message_type(field=self.field, filter_boolean=self.inner_message_type(value=self.value, operator=self.operator))


class ArrayFilter(Filter):
    """
    Filter for array comparisons
    """
    contains_ = FILTER_ARRAY_OPERATOR_CONTAINS
    notcontains_ = FILTER_ARRAY_OPERATOR_NOT_CONTAINS
    value_type_ = str

    def __init__(self, field: Optional[Message], disjunction_message_type: Type[Message], conjunction_message_type: Type[Message], message_type: Type[Message], inner_message_type: Optional[Type[Message]] = FilterArray, filters: Optional[List[List["Filter"]]] = None, value=None, operator=None):
        super().__init__(field, disjunction_message_type, conjunction_message_type, message_type, inner_message_type, filters, value, operator)

    def to_basic_message(self) -> Message:
        return self.message_type(field=self.field, filter_array=self.inner_message_type(value=self.value, operator=self.operator))


class DurationFilter(Filter):
    """
    Filter for duration comparison
    """
    eq_ = FILTER_DURATION_OPERATOR_EQUAL
    ne_ = FILTER_DURATION_OPERATOR_NOT_EQUAL
    lt_ = FILTER_DURATION_OPERATOR_SHORTER_THAN
    le_ = FILTER_DURATION_OPERATOR_SHORTER_THAN_OR_EQUAL
    gt_ = FILTER_DURATION_OPERATOR_LONGER_THAN
    ge_ = FILTER_DURATION_OPERATOR_LONGER_THAN_OR_EQUAL
    value_type_ = duration.Duration

    def __init__(self, field: Optional[Message], disjunction_message_type: Type[Message], conjunction_message_type: Type[Message], message_type: Type[Message], inner_message_type: Optional[Type[Message]] = FilterDuration, filters: Optional[List[List["Filter"]]] = None, value=None, operator=None):
        super().__init__(field, disjunction_message_type, conjunction_message_type, message_type, inner_message_type, filters, value, operator)

    def to_basic_message(self) -> Message:
        return self.message_type(field=self.field, filter_duration=self.inner_message_type(value=self.value, operator=self.operator))
