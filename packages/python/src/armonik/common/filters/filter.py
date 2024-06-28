from __future__ import annotations

import json
from abc import abstractmethod, ABC
from datetime import datetime, timedelta
from typing import Optional, Union, Any, List, Protocol, Dict, Type, cast

from google.protobuf.message import Message

# noinspection PyUnresolvedReferences
from google.protobuf.duration_pb2 import Duration

# noinspection PyUnresolvedReferences
from google.protobuf.timestamp_pb2 import Timestamp

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

SimpleOperator = int


class BasicMessageType(Protocol):
    """
    Message type that requires a "field" kwarg
    """

    def __call__(self, *, field, **kwargs) -> Message: ...


class CombinationMessageType(Protocol):
    """
    Message type for the ands and ors
    """

    def __call__(self, **kwargs) -> Message: ...


class InnerMessageType(Protocol):
    """
    Message type for the value and operator
    """

    def __call__(self, *, value: Any, operator: SimpleOperator) -> Message: ...


class FilterError(ValueError):
    """
    Exception raised when there is an error related to filters
    """

    def __init__(self, filter_instance: Optional[Filter], message: str):
        self.filter = filter_instance
        self.message = message

    def __str__(self):
        return f"Filter: {str(self.filter)} Error: {self.message}"


class Filter:
    """
    Base class of all filters
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

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: CombinationMessageType,
        conjunction_message_type: CombinationMessageType,
        message_type: Union[BasicMessageType, CombinationMessageType],
        inner_message_type: Optional[InnerMessageType],
        operator: Union[SimpleOperator, None] = None,
        value: Any = None,
    ):
        """
        Init a new filter
        Notes: This class should not be instanced directly. Always use the proxies in armonik.client.filters
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
        Creates a disjunction from the current Filter types and a list of lists of filters
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
        if not (
            self.disjunction_message_type == other.disjunction_message_type
            and self.conjunction_message_type == other.conjunction_message_type
        ):
            raise ValueError(
                f"{self.__class__.__name__} is not compatible with "
                f"{other.__class__.__name__} as they have different disjunction/conjunction types"
            )

    def _is_empty(self) -> bool:
        return self.value is None or (
            isinstance(self.value, list)
            and (
                len(self.value) == 0
                or (isinstance(self.value[0], list) and len(self.value[0]) == 0)
            )
        )

    def _is_disjunction(self) -> bool:
        return self.message_type == self.disjunction_message_type

    def to_disjunction(self) -> Filter:
        """
        Wraps the current Filter into a disjunction. Does nothing if the current Filter is already a disjunction.
        Returns:
            Disjunction Filter
        """
        if self._is_empty():
            raise FilterError(self, "Empty filter")
        if isinstance(self.operator, SimpleOperator):
            return self._disjunction([[self]])
        return self

    def __and__(self, other: Filter) -> Filter:
        self._check_compatible(other)
        """
        (a | (b&c)) & (d | (e&f)) = a&d | a&e&f | b&c&d | b&c&e&f
        """
        return self._disjunction(
            [
                c1 + c2
                for c2 in other.to_disjunction().value
                for c1 in self.to_disjunction().value
            ]
        )

    def __or__(self, other: Filter) -> Filter:
        self._check_compatible(other)
        return self._disjunction(
            self.to_disjunction().value + other.to_disjunction().value
        )

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
        if self.operator is None:
            if self._is_empty():
                raise FilterError(
                    self, "Cannot invert filter without a value or operator"
                )
            try:
                new_filter = self._disjunction([[~c] for c in self.value[0]])
                for conj in self.value[1:]:
                    new_filter &= self._disjunction([[~c] for c in conj])
                return new_filter
            except FilterError as e:
                raise FilterError(
                    self,
                    f"Could not invert because of an error in subfilter inversion : {str(e)}",
                )
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
        raise FilterError(self, msg)

    def _sanitize_value(self, value: Any) -> Any:
        if self.__class__.value_type_ is None or isinstance(
            value, self.__class__.value_type_
        ):
            return value
        raise FilterError(
            None,
            f"Expected value type {str(self.__class__.value_type_)} for field {str(self.field)}, got {str(type(value))} instead",
        )

    def _check(self, value: Any, operator: Any, operator_str: str = "") -> Filter:
        """
        Internal function to create a new filter from the current filter with a different value and/or operator
        Args:
            value: Value of the new filter
            operator: Operator of the new filter
            operator_str: Optional string for error message clarification

        Returns:
            new filter with the given value/operator

        Raises:
            FilterError if the given operator is not available for the given class
        """
        if self._is_disjunction():
            raise FilterError(
                self, "Cannot apply operator to a disjunction or a conjunction"
            )
        if operator is None:
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
        if not self._is_disjunction():
            return {
                "field": str(self.field),
                "value": str(self.value),
                "operator": str(self.operator),
            }
        return (
            {
                "or": [
                    (
                        {"and": [c.to_dict() for c in conj]}
                        if len(conj) > 1
                        else conj[0].to_dict()
                    )
                    for conj in self.value
                ]
            }
            if len(self.value) > 1
            else (
                (
                    {"and": [c.to_dict() for c in self.value[0]]}
                    if len(self.value[0]) > 1
                    else self.value[0][0].to_dict()
                )
                if not self._is_empty()
                else {}
            )
        )

    def __str__(self) -> str:
        return json.dumps(self.to_dict())

    @abstractmethod
    def to_basic_message(self) -> Message:
        raise NotImplementedError(
            "Should not be implemented here. Use a subclass instead"
        )

    def to_message(self) -> Message:
        if not self._is_disjunction():
            return self.to_disjunction().to_message()

        def to_conjunction_message(conj: List[Filter]) -> Message:
            conj_raw = self.conjunction_message_type()
            getattr(conj_raw, "and").extend(f.to_basic_message() for f in conj)
            return conj_raw

        raw = self.disjunction_message_type()
        getattr(raw, "or").extend(to_conjunction_message(conj) for conj in self.value)
        return raw


class StringFilter(Filter):
    """
    Filter for string comparisons
    """

    eq_ = FILTER_STRING_OPERATOR_EQUAL
    ne_ = FILTER_STRING_OPERATOR_NOT_EQUAL
    contains_ = FILTER_STRING_OPERATOR_CONTAINS
    notcontains_ = FILTER_STRING_OPERATOR_NOT_CONTAINS
    value_type_ = str

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: CombinationMessageType,
        conjunction_message_type: CombinationMessageType,
        message_type: Union[BasicMessageType, CombinationMessageType],
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
        return self._check(value, FILTER_STRING_OPERATOR_STARTS_WITH, "startswith")

    def endswith(self, value: str) -> "StringFilter":
        return self._check(value, FILTER_STRING_OPERATOR_ENDS_WITH, "endswith")

    def to_basic_message(self) -> Message:
        return self.message_type(
            field=self.field,
            filter_string=self.inner_message_type(
                value=self.value, operator=self.operator
            ),
        )

    def __repr__(self) -> str:
        return f'{str(self.field)} {str(self.operator)} "{str(self.value)}"'


class StatusFilter(Filter):
    """
    Filter for status comparison
    """

    eq_ = FILTER_STATUS_OPERATOR_EQUAL
    ne_ = FILTER_STATUS_OPERATOR_NOT_EQUAL

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: CombinationMessageType,
        conjunction_message_type: CombinationMessageType,
        message_type: Union[BasicMessageType, CombinationMessageType],
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

    def to_basic_message(self) -> Message:
        return self.message_type(
            field=self.field,
            filter_status=self.inner_message_type(
                value=self.value, operator=self.operator
            ),
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

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: CombinationMessageType,
        conjunction_message_type: CombinationMessageType,
        message_type: Union[BasicMessageType, CombinationMessageType],
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
        raise FilterError(
            self,
            f"Expected value type {datetime.__class__.__name__} or {Timestamp.__class__.__name__}"
            f"for field {str(self.field)}, got {str(type(value))} instead",
        )

    def to_basic_message(self) -> Message:
        return self.message_type(
            field=self.field,
            filter_date=self.inner_message_type(
                value=self.value, operator=self.operator
            ),
        )


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

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: CombinationMessageType,
        conjunction_message_type: CombinationMessageType,
        message_type: Union[BasicMessageType, CombinationMessageType],
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

    def to_basic_message(self) -> Message:
        return self.message_type(
            field=self.field,
            filter_number=self.inner_message_type(
                value=self.value, operator=self.operator
            ),
        )


class BooleanFilter(Filter):
    """
    Filter for boolean comparison
    """

    eq_ = FILTER_BOOLEAN_OPERATOR_IS
    value_type_ = bool

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: CombinationMessageType,
        conjunction_message_type: CombinationMessageType,
        message_type: Union[BasicMessageType, CombinationMessageType],
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
        return self.__eq__(not value)

    def __invert__(self) -> BooleanFilter:
        return self.__eq__(not self.value)

    def to_basic_message(self) -> Message:
        return self.message_type(
            field=self.field,
            filter_boolean=self.inner_message_type(
                value=self.value, operator=self.operator
            ),
        )


class ArrayFilter(Filter):
    """
    Filter for array comparisons
    """

    contains_ = FILTER_ARRAY_OPERATOR_CONTAINS
    notcontains_ = FILTER_ARRAY_OPERATOR_NOT_CONTAINS
    value_type_ = str

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: CombinationMessageType,
        conjunction_message_type: CombinationMessageType,
        message_type: Union[BasicMessageType, CombinationMessageType],
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

    def to_basic_message(self) -> Message:
        return self.message_type(
            field=self.field,
            filter_array=self.inner_message_type(
                value=self.value, operator=self.operator
            ),
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

    def __init__(
        self,
        field: Optional[Message],
        disjunction_message_type: CombinationMessageType,
        conjunction_message_type: CombinationMessageType,
        message_type: Union[BasicMessageType, CombinationMessageType],
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
        raise FilterError(
            self,
            f"Expected value type {timedelta.__class__.__name__} or {Duration.__class__.__name__}"
            f"for field {str(self.field)}, got {str(type(value))} instead",
        )

    def to_basic_message(self) -> Message:
        return self.message_type(
            field=self.field,
            filter_duration=self.inner_message_type(
                value=self.value, operator=self.operator
            ),
        )


class FilterWrapper(ABC):
    def __init__(
        self,
        disjunction_message_type: Type[Message],
        conjunction_message_type: Type[Message],
        message_type: Type[Message],
        status_type: Optional[Type[Message]] = None,
    ):
        self.disjunction_message_type = cast(
            CombinationMessageType, disjunction_message_type
        )
        self.conjunction_message_type = cast(
            CombinationMessageType, conjunction_message_type
        )
        self.message_type = cast(BasicMessageType, message_type)
        self.status_type = cast(InnerMessageType, status_type)

    def _string(self, field: Message) -> StringFilter:
        return StringFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.message_type,
        )

    def _bool(self, field: Message) -> BooleanFilter:
        return BooleanFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.message_type,
        )

    def _array(self, field: Message) -> ArrayFilter:
        return ArrayFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.message_type,
        )

    def _duration(self, field: Message) -> DurationFilter:
        return DurationFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.message_type,
        )

    def _date(self, field: Message) -> DateFilter:
        return DateFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.message_type,
        )

    def _number(self, field: Message) -> NumberFilter:
        return NumberFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.message_type,
        )

    def _status(self, field: Message) -> StatusFilter:
        return StatusFilter(
            field,
            self.disjunction_message_type,
            self.conjunction_message_type,
            self.message_type,
            self.status_type,
        )
