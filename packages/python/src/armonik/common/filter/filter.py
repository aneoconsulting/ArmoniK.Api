from __future__ import annotations

import json
from abc import abstractmethod, ABC
from datetime import datetime, timedelta
from enum import Enum, auto
from typing import Optional, Union, Any, List, Dict, Generic, TypeVar, overload, Type, Tuple

# noinspection PyUnresolvedReferences
from google.protobuf.duration_pb2 import Duration
from google.protobuf.message import Message

# noinspection PyUnresolvedReferences
from google.protobuf.timestamp_pb2 import Timestamp

from ._message_types import (
    BasicMessageType,
    InnerMessageType,
    SimpleOperator,
    DisjunctionType,
    ConjunctionType,
)
from ..helpers import datetime_to_timestamp, timedelta_to_duration
from ...protogen.common.filters_common_pb2 import (
    FILTER_STRING_OPERATOR_EQUAL,
    FILTER_STRING_OPERATOR_NOT_EQUAL,
    FILTER_STRING_OPERATOR_CONTAINS,
    FILTER_STRING_OPERATOR_NOT_CONTAINS,
    FilterString,
    FILTER_STRING_OPERATOR_STARTS_WITH,
    FILTER_STRING_OPERATOR_ENDS_WITH,
    FILTER_STATUS_OPERATOR_EQUAL,
    FILTER_STATUS_OPERATOR_NOT_EQUAL,
    FILTER_DATE_OPERATOR_EQUAL,
    FILTER_DATE_OPERATOR_NOT_EQUAL,
    FILTER_DATE_OPERATOR_BEFORE,
    FILTER_DATE_OPERATOR_BEFORE_OR_EQUAL,
    FILTER_DATE_OPERATOR_AFTER,
    FILTER_DATE_OPERATOR_AFTER_OR_EQUAL,
    FILTER_NUMBER_OPERATOR_EQUAL,
    FILTER_NUMBER_OPERATOR_NOT_EQUAL,
    FILTER_NUMBER_OPERATOR_LESS_THAN,
    FILTER_NUMBER_OPERATOR_LESS_THAN_OR_EQUAL,
    FILTER_NUMBER_OPERATOR_GREATER_THAN,
    FILTER_NUMBER_OPERATOR_GREATER_THAN_OR_EQUAL,
    FilterDate,
    FilterNumber,
    FILTER_BOOLEAN_OPERATOR_IS,
    FilterBoolean,
    FILTER_ARRAY_OPERATOR_CONTAINS,
    FILTER_ARRAY_OPERATOR_NOT_CONTAINS,
    FilterArray,
    FILTER_DURATION_OPERATOR_EQUAL,
    FILTER_DURATION_OPERATOR_NOT_EQUAL,
    FILTER_DURATION_OPERATOR_SHORTER_THAN,
    FILTER_DURATION_OPERATOR_SHORTER_THAN_OR_EQUAL,
    FILTER_DURATION_OPERATOR_LONGER_THAN,
    FILTER_DURATION_OPERATOR_LONGER_THAN_OR_EQUAL,
    FilterDuration,
)


class FilterError(ValueError):
    """
    Exception raised when there is an error related to filter
    """

    def __init__(self, filter_instance: Optional[Filter], message: str):
        """

        Args:
            filter_instance: Instance of the filter
            message: Accompanying message
        """
        self.filter = filter_instance
        self.message = message

    def __str__(self):
        return f"Filter: {str(self.filter)} Error: {self.message}"


eq_special = object()
ne_special = object()


class Filter:
    """
    Base class of all filters.
    How to use it :
    A filter should not be created directly by the end user. Instead, users should use the class static properties to indicate what filter they want to use.
    For example, if a user wants to list tasks that are in session "xxx", then they should use :
    Task.session_id == "xxx"
    Users can combine filters using the binary operators (& | ~).

    When combining filters together, the new filter combination is always kept in the disjunctive normal form to be compatible with ArmoniK.
    """

    eq_ = eq_special  # Special case
    ne_ = ne_special  # Special case
    lt_ = None
    le_ = None
    gt_ = None
    ge_ = None
    contains_ = None
    notcontains_ = None
    value_type_ = None
    inner_message_type_attr_ = None

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: Union[BasicMessageType, DisjunctionType],
        inner_message_type: Optional[InnerMessageType],
        operator: Union[SimpleOperator, None] = None,
        value: Any = None,
    ):
        """
        Init a new filter
        Notes: This class should not be instanced directly. Always use the proxies in armonik.common.filter
        Args:
            field: gRPC Message field
            disjunction_message_type: Message type of the logical OR
            conjunction_message_type: Message type of the logical AND
            message_type: Filter gRPC message type
            inner_message_type: Filter type gRPC message type
            operator: Filter operator
            value: Value of the filter
        """
        self.field = field
        self.disjunction_message_type = disjunction_message_type
        self.conjunction_message_type = conjunction_message_type
        self.message_type = message_type
        self.inner_message_type = inner_message_type
        self.operator = operator
        self.value = value

    def _disjunction(self, filters: List[List[Filter]]) -> Filter:
        """
        Creates a disjunction from the current Filter types and a list of lists of filter
        Args:
            filters: List of Lists of Filters (OR of ANDs)

        Returns:
            New Filter instance as disjunction
        """
        return Filter(
            None,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.disjunction_message_type,
            None,
            None,
            filters,
        )

    def _check_compatible(self, other: Filter):
        """
        Checks if the given Filter is compatible with the current Filter. Raises ValueError if they are not compatible.
        Args:
            other: Other Filter

        Returns:
            None
        """
        if not isinstance(other, Filter):
            msg = (
                f"{str(self)} is not compatible with " f"{str(other)} as the latter is not a Filter"
            )
            raise ValueError(msg)
        if not (
            self.disjunction_message_type == other.disjunction_message_type
            and self.conjunction_message_type == other.conjunction_message_type
        ):
            msg = (
                f"{str(self)} is not compatible with "
                f"{str(other)} as they apply to different object types"
            )
            raise ValueError(msg)

    def _is_empty(self) -> bool:
        """
        Checks if the current Filter is empty
        Returns:
            True if the current Filter is empty
        """
        return self.value is None or (
            isinstance(self.value, list)
            and (
                len(self.value) == 0
                or (isinstance(self.value[0], list) and len(self.value[0]) == 0)
            )
        )

    def _is_disjunction(self) -> bool:
        """
        Checks if the current Filter is a disjunction
        Returns:
            True if the current Filter is a disjunction
        """
        return self.message_type == self.disjunction_message_type

    def to_disjunction(self) -> Filter:
        """
        Wraps the current Filter into a disjunction. Does nothing if the current Filter is already a disjunction.
        Returns:
            Disjunction Filter
        Raises:
            FilterError if the filter is empty
        """
        if self._is_empty():
            raise FilterError(self, "Empty filter")
        if isinstance(self.operator, SimpleOperator):
            return self._disjunction([[self]])
        return self

    def __and__(self, other: Filter) -> Filter:
        """
        Logical and
        """
        self._check_compatible(other)
        # (a | (b&c)) & (d | (e&f)) = a&d | a&e&f | b&c&d | b&c&e&f
        return self._disjunction(
            [c1 + c2 for c2 in other.to_disjunction().value for c1 in self.to_disjunction().value]
        )

    def __mul__(self, other: Filter) -> Filter:
        """
        Same as A & B
        """
        return self & other

    def __or__(self, other: Filter) -> Filter:
        """
        Logical or
        """
        self._check_compatible(other)
        # (a | b&c) | (d | e&f) = a | b&c | d | e&f
        return self._disjunction(self.to_disjunction().value + other.to_disjunction().value)

    def __add__(self, other: Filter) -> Filter:
        """
        Same as Logical or
        """
        return self | other

    def __eq__(self, value) -> Filter:
        """
        Creates a Filter which checks for the equality of the property compared to the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, self.__class__.eq_, "==")

    def __ne__(self, value) -> Filter:
        """
        Creates a Filter which checks for the inequality of the property compared to the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, self.__class__.ne_, "!=")

    def __lt__(self, value) -> Filter:
        """
        Creates a Filter which checks that the object's value is less than the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, self.__class__.lt_, "<")

    def __le__(self, value) -> Filter:
        """
        Creates a Filter which checks that the object's value is less than or equal to the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, self.__class__.le_, "<=")

    def __gt__(self, value) -> Filter:
        """
        Creates a Filter which checks that the object's value is greater than to the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, self.__class__.gt_, ">")

    def __ge__(self, value) -> Filter:
        """
        Creates a Filter which checks that the object's value is greater than or equal to the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, self.__class__.ge_, ">=")

    def contains(self, value) -> Filter:
        """
        Creates a Filter which checks that the object's value contains the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, self.__class__.contains_, "contains")

    def __invert__(self) -> Filter:
        """
        Logical not
        """
        if self.operator is None:
            # No operator
            if self._is_empty():
                raise FilterError(self, "Cannot invert filter without a value or operator")
            try:
                # The filter is a combination
                # ~(a | (b & c)) => ~a & (~b | ~c) => ~a&~b | ~a&~c
                new_filter = self._disjunction([[~c] for c in self.value[0]])
                for conj in self.value[1:]:
                    new_filter &= self._disjunction([[~c] for c in conj])
                return new_filter
            except FilterError as e:
                msg = f"Could not invert because of an error in subfilter inversion : {str(e)}"
                raise FilterError(self, msg)
        # Invert the operation
        if self.operator == self.__class__.eq_:
            return self._change_operation(self.value, self.__class__.ne_, "!=", True)
        if self.operator == self.__class__.ne_:
            return self._change_operation(self.value, self.__class__.eq_, "==", True)
        if self.operator == self.__class__.lt_:
            return self._change_operation(self.value, self.__class__.ge_, ">=", True)
        if self.operator == self.__class__.le_:
            return self._change_operation(self.value, self.__class__.gt_, ">", True)
        if self.operator == self.__class__.gt_:
            return self._change_operation(self.value, self.__class__.le_, "<=", True)
        if self.operator == self.__class__.ge_:
            return self._change_operation(self.value, self.__class__.lt_, "<", True)
        if self.operator == self.__class__.contains_:
            return self._change_operation(
                self.value, self.__class__.notcontains_, "not_contains", True
            )
        if self.operator == self.__class__.notcontains_:
            return self._change_operation(self.value, self.__class__.contains_, "contains", True)
        msg = f"{self.__class__.__name__} operator {str(self.operator)} for field {str(self.field)} has no inverted equivalent"
        raise FilterError(self, msg)

    def __neg__(self) -> Filter:
        """
        Same as Logical not
        """
        return ~self

    def __xor__(self, other: Filter) -> Filter:
        """
        Logical xor
        """
        return (self & (~other)) | ((~self) & other)

    def _sanitize_value(self, value: Any) -> Any:
        """
        Takes an input value and sanitizes it to be compatible with the filter.
        If the type is incompatible, throws a FilterError
        Args:
            value: Value to be sanitized

        Returns:
            Sanitized value
        """
        if self.__class__.value_type_ is None or isinstance(value, self.__class__.value_type_):
            return value
        msg = f"Expected value type {str(self.__class__.value_type_)} for field {str(self.field)}, got {str(type(value))} instead"
        raise FilterError(None, msg)

    def _change_operation(
        self, value: Any, operator: Any, operator_str: str = "", override: bool = False
    ) -> Filter:
        """
        Internal function to create a new filter from the current filter with a different value and/or operator
        Args:
            value: Value of the new filter
            operator: Operator of the new filter
            operator_str: Optional string for error message clarification
            override: If set to true, bypasses the check for reassignment
        Returns:
            new filter with the given value/operator

        Raises:
            FilterError if the given operator is not available for the given class
        """
        if not override:
            if self.value is not None:
                if isinstance(value, bool):
                    if operator is self.__class__.eq_ and self.__class__.eq_ is not None:
                        return self if value else ~self
                    if operator is self.__class__.ne_ and self.__class__.ne_ is not None:
                        return ~self if value else self
                if self._is_disjunction():
                    raise FilterError(self, "Cannot apply operator to a filter combination")
                raise FilterError(self, "Cannot apply operator to an already defined filter")
        if operator is None or self._is_disjunction():
            msg = f"Operator {operator_str} is not available for {self.__class__.__name__}"
            raise FilterError(self, msg)
        return self.__class__(
            self.field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.message_type,
            self.inner_message_type,
            operator,
            self._sanitize_value(value),
        )

    def to_dict(self) -> Dict[str, Any]:
        """
        Transforms the filter in a dictionary describing it
        Returns:
            Dictionary representing the filter in a disjunctive normal form
        """
        if not self._is_disjunction() or self.value is None:
            return {
                "field": str(self.field),
                "value": str(self.value),
                "operator": str(self.operator),
            }
        if len(self.value) > 1:
            return {
                "or": [
                    ({"and": [c.to_dict() for c in conj]} if len(conj) > 1 else conj[0].to_dict())
                    for conj in self.value
                ]
            }
        return (
            (
                {"and": [c.to_dict() for c in self.value[0]]}
                if len(self.value[0]) > 1
                else self.value[0][0].to_dict()
            )
            if not self._is_empty()
            else {}
        )

    def __str__(self) -> str:
        return json.dumps(self.to_dict())

    def to_basic_message(self) -> BasicMessageType:
        """
        Converts the filter into its base gRPC message
        Returns:
            Base gRPC message
        """
        if self.__class__.inner_message_type_attr_ is None:
            raise FilterError(
                self,
                "Cannot use an empty filter in combination with others. "
                "Did you forget to write the condition ?",
            )
        return self.message_type(
            field=self.field,
            **{
                self.__class__.inner_message_type_attr_: self.inner_message_type(
                    value=self.value, operator=self.operator
                )
            },
        )

    def to_message(self) -> DisjunctionType:
        """
        Converts the filter into its gRPC disjunctive message
        Returns:
            Disjunctive gRPC message
        """
        if not self._is_disjunction():
            # Convert the message to a disjunction if it's not the case
            return self.to_disjunction().to_message()

        def to_conjunction_message(conj: List[Filter]) -> ConjunctionType:
            conj_raw = self.conjunction_message_type()
            # Need to use getattr because and is a reserved name
            getattr(conj_raw, "and").extend(f.to_basic_message() for f in conj)
            return conj_raw

        raw = self.disjunction_message_type()
        # Need to use getattr because or is a reserved name
        getattr(raw, "or").extend(
            (to_conjunction_message(conj) for conj in self.value) if self.value is not None else []
        )
        return raw

    def __bool__(self):
        raise FilterError(
            self,
            "Filters cannot be transformed into booleans. "
            "You may see this error if you try to combine filters with 'or', 'and', 'in', or 'not'. "
            "Use '|', '&', '.contains' or '~' instead.",
        )


class StringFilter(Filter):
    """
    Filter for string comparisons
    """

    eq_ = FILTER_STRING_OPERATOR_EQUAL
    ne_ = FILTER_STRING_OPERATOR_NOT_EQUAL
    contains_ = FILTER_STRING_OPERATOR_CONTAINS
    notcontains_ = FILTER_STRING_OPERATOR_NOT_CONTAINS
    value_type_ = str
    inner_message_type_attr_ = "filter_string"

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: Union[BasicMessageType, DisjunctionType],
        inner_message_type: Optional[InnerMessageType] = FilterString,
        operator: Union[SimpleOperator, None] = None,
        value: Any = None,
    ):
        super().__init__(
            field,
            disjunction_message_type,
            conjunction_message_type,
            message_type,
            inner_message_type,
            operator,
            value,
        )

    def startswith(self, value: str) -> "StringFilter":
        """
        Creates a Filter which checks that the object's value starts with the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, FILTER_STRING_OPERATOR_STARTS_WITH, "startswith")

    def endswith(self, value: str) -> "StringFilter":
        """
        Creates a Filter which checks that the object's value ends with the value
        Args:
            value: Value to check against

        Returns:
            New Filter instance
        """
        return self._change_operation(value, FILTER_STRING_OPERATOR_ENDS_WITH, "endswith")

    def __repr__(self) -> str:
        return f'{str(self.field)} {str(self.operator)} "{str(self.value)}"'


class StatusFilter(Filter):
    """
    Filter for status comparison
    """

    eq_ = FILTER_STATUS_OPERATOR_EQUAL
    ne_ = FILTER_STATUS_OPERATOR_NOT_EQUAL
    inner_message_type_attr_ = "filter_status"

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: Union[BasicMessageType, DisjunctionType],
        inner_message_type: InnerMessageType,
        operator: Union[SimpleOperator, None] = None,
        value: Any = None,
    ):
        super().__init__(
            field,
            disjunction_message_type,
            conjunction_message_type,
            message_type,
            inner_message_type,
            operator,
            value,
        )


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
    inner_message_type_attr_ = "filter_date"

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: Union[BasicMessageType, DisjunctionType],
        inner_message_type: InnerMessageType = FilterDate,
        operator: Union[SimpleOperator, None] = None,
        value: Any = None,
    ):
        super().__init__(
            field,
            disjunction_message_type,
            conjunction_message_type,
            message_type,
            inner_message_type,
            operator,
            value,
        )

    def _sanitize_value(self, value: Any) -> Any:
        if isinstance(value, datetime):
            return datetime_to_timestamp(value)
        if isinstance(value, Timestamp):
            return value
        msg = (
            f"Expected value type {datetime.__class__.__name__} or {Timestamp.__class__.__name__}"
            f"for field {str(self.field)}, got {str(type(value))} instead"
        )
        raise FilterError(self, msg)


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
    inner_message_type_attr_ = "filter_number"

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: Union[BasicMessageType, DisjunctionType],
        inner_message_type: InnerMessageType = FilterNumber,
        operator: Union[SimpleOperator, None] = None,
        value: Any = None,
    ):
        super().__init__(
            field,
            disjunction_message_type,
            conjunction_message_type,
            message_type,
            inner_message_type,
            operator,
            value,
        )


class BooleanFilter(Filter):
    """
    Filter for boolean comparison
    """

    eq_ = FILTER_BOOLEAN_OPERATOR_IS
    value_type_ = bool
    inner_message_type_attr_ = "filter_boolean"

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: Union[BasicMessageType, DisjunctionType],
        inner_message_type: InnerMessageType = FilterBoolean,
        operator: Union[SimpleOperator, None] = FILTER_BOOLEAN_OPERATOR_IS,
        value: Any = True,
    ):
        super().__init__(
            field,
            disjunction_message_type,
            conjunction_message_type,
            message_type,
            inner_message_type,
            operator,
            value,
        )

    def __ne__(self, value: bool) -> BooleanFilter:
        return self._change_operation(not value, self.__class__.eq_, "!=", True)

    def __invert__(self) -> BooleanFilter:
        return self._change_operation(not self.value, self.__class__.eq_, "==", True)


class ArrayFilter(Filter):
    """
    Filter for array comparisons
    """

    contains_ = FILTER_ARRAY_OPERATOR_CONTAINS
    notcontains_ = FILTER_ARRAY_OPERATOR_NOT_CONTAINS
    value_type_ = str
    inner_message_type_attr_ = "filter_array"

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: Union[BasicMessageType, DisjunctionType],
        inner_message_type: InnerMessageType = FilterArray,
        operator: Union[SimpleOperator, None] = None,
        value: Any = None,
    ):
        super().__init__(
            field,
            disjunction_message_type,
            conjunction_message_type,
            message_type,
            inner_message_type,
            operator,
            value,
        )


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
    value_type_ = Duration
    inner_message_type_attr_ = "filter_duration"

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: Union[BasicMessageType, DisjunctionType],
        inner_message_type: InnerMessageType = FilterDuration,
        operator: Union[SimpleOperator, None] = None,
        value: Any = None,
    ):
        super().__init__(
            field,
            disjunction_message_type,
            conjunction_message_type,
            message_type,
            inner_message_type,
            operator,
            value,
        )

    def _sanitize_value(self, value: Any) -> Any:
        if isinstance(value, timedelta):
            return timedelta_to_duration(value)
        if isinstance(value, Duration):
            return value
        msg = (
            f"Expected value type {timedelta.__class__.__name__} or {Duration.__class__.__name__}"
            f"for field {str(self.field)}, got {str(type(value))} instead"
        )
        raise FilterError(self, msg)


class FType(Enum):
    """
    Enum for the type of a filter. Used internally.
    """

    UNKNOWN = auto()
    NA = auto()
    NUM = auto()
    STR = auto()
    ARRAY = auto()
    DURATION = auto()
    DATE = auto()
    STATUS = auto()
    BOOL = auto()


def _raise(field):
    """
    The field is unknown in the class
    Args:
        field: Unknown field

    Returns:

    """
    msg = f"Unknown field {field}"
    raise ValueError(msg)


def _na(field):
    msg = f"Field {field} is not available as a filter"
    raise FilterError(None, msg)


class FilterWrapper(Filter, ABC):
    """
    Wraps the filter creation to alleviate repetitions
    """

    _fields: Dict[str, Tuple[FType, Optional[Any]]] = {}

    def __init__(
        self,
        disjunction_message_type: DisjunctionType,
        conjunction_message_type: ConjunctionType,
        message_type: BasicMessageType,
        status_type: Optional[InnerMessageType] = None,
    ):
        super().__init__(
            None,
            disjunction_message_type,
            conjunction_message_type,
            disjunction_message_type,
            status_type,
        )
        self._vtable = {
            FType.UNKNOWN: _raise,
            FType.NA: _na,
            FType.NUM: self._number,
            FType.STR: self._string,
            FType.ARRAY: self._array,
            FType.DURATION: self._duration,
            FType.DATE: self._date,
            FType.STATUS: self._status,
            FType.BOOL: self._bool,
        }
        self.basic_message_type = message_type

    def _string(self, field: Message) -> StringFilter:
        """
        Creates a new string filter on the given field
        Args:
            field: field to filter against

        Returns:
            New string filter on the given field
        """
        return StringFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.basic_message_type,
        )

    def _bool(self, field: Message) -> BooleanFilter:
        """
        Creates a new boolean filter on the given field
        Args:
            field: field to filter against

        Returns:
            New boolean filter on the given field
        """
        return BooleanFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.basic_message_type,
        )

    def _array(self, field: Message) -> ArrayFilter:
        """
        Creates a new array filter on the given field
        Args:
            field: field to filter against

        Returns:
            New array filter on the given field
        """
        return ArrayFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.basic_message_type,
        )

    def _duration(self, field: Message) -> DurationFilter:
        """
        Creates a new duration filter on the given field
        Args:
            field: field to filter against

        Returns:
            New duration filter on the given field

        """
        return DurationFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.basic_message_type,
        )

    def _date(self, field: Message) -> DateFilter:
        """
        Creates a new date filter on the given field
        Args:
            field:  field to filter against

        Returns:
            New date filter on the given field

        """
        return DateFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.basic_message_type,
        )

    def _number(self, field: Message) -> NumberFilter:
        """
        Creates a new number filter on the given field
        Args:
            field:  field to filter against

        Returns:
            New number filter on the given field

        """
        return NumberFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.basic_message_type,
        )

    def _status(self, field: Message) -> StatusFilter:
        """
        Creates a new status filter on the given field
        Args:
            field:  field to filter against

        Returns:
            New status filter on the given field

        """
        return StatusFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.basic_message_type,
            self.inner_message_type,
        )

    @abstractmethod
    def _build_field(self, field: Any) -> Message:
        """
        Build the gRPC field object
        Args:
            field: base field enum value

        Returns:
            Field message
        """
        ...

    def __call__(self, field_name: str) -> Filter:
        """
        Get the Filter associated with the field name
        Args:
            field_name: Field name

        Returns:
            Filter associated with the field name
        """
        ftype, field_value = self.__class__._fields.get(field_name, (FType.UNKNOWN, field_name))
        return self._vtable[ftype](
            self._build_field(field_value)
            if ftype not in (FType.UNKNOWN, FType.NA)
            else field_value
        )


AttributeType = TypeVar("AttributeType")


class FilterDescriptor(Generic[AttributeType]):
    """
    Descriptor used to declare a filter
    """

    def __init__(self, wrapper: FilterWrapper):
        self.wrapper = wrapper

    def __set_name__(self, owner: Type, name: str):
        self.name = name
        self._name = "_" + name

    @overload
    def __get__(self, instance: None, owner: Type) -> Filter: ...

    @overload
    def __get__(self, instance: object, owner: Type) -> AttributeType: ...

    def __get__(
        self, instance: Optional[object], owner: Optional[Type]
    ) -> Union[Filter, AttributeType]:
        # When called statically, returns the filter
        if instance is None:
            return self.wrapper(self.name)
        # Otherwise gets the object's hidden property
        return getattr(instance, self._name)

    def __set__(self, instance: object, value: AttributeType):
        setattr(instance, self._name, value)
