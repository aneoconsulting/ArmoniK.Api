from typing import Any

from .filter import FilterWrapper, StringFilter, StatusFilter, DateFilter, NumberFilter
from ...protogen.common.results_fields_pb2 import (
    RESULT_RAW_ENUM_FIELD_RESULT_ID,
    RESULT_RAW_ENUM_FIELD_STATUS,
    RESULT_RAW_ENUM_FIELD_COMPLETED_AT,
    RESULT_RAW_ENUM_FIELD_CREATED_AT,
    RESULT_RAW_ENUM_FIELD_OWNER_TASK_ID,
    RESULT_RAW_ENUM_FIELD_SESSION_ID,
    RESULT_RAW_ENUM_FIELD_NAME,
    RESULT_RAW_ENUM_FIELD_SIZE,
    ResultField,
    ResultRawField,
)
from ...protogen.common.results_filters_pb2 import (
    Filters,
    FiltersAnd,
    FilterField,
    FilterStatus,
)


def _raw_field(field: Any) -> ResultField:
    return ResultField(result_raw_field=ResultRawField(field=field))


class ResultFilter(FilterWrapper):
    """
    Filter for results
    """

    def __init__(self):
        super().__init__(Filters, FiltersAnd, FilterField, FilterStatus)

    @property
    def session_id(self) -> StringFilter:
        return self._string(_raw_field(RESULT_RAW_ENUM_FIELD_SESSION_ID))

    @property
    def name(self) -> StringFilter:
        return self._string(_raw_field(RESULT_RAW_ENUM_FIELD_NAME))

    @property
    def owner_task_id(self) -> StringFilter:
        return self._string(_raw_field(RESULT_RAW_ENUM_FIELD_OWNER_TASK_ID))

    @property
    def status(self) -> StatusFilter:
        return self._status(_raw_field(RESULT_RAW_ENUM_FIELD_STATUS))

    @property
    def created_at(self) -> DateFilter:
        return self._date(_raw_field(RESULT_RAW_ENUM_FIELD_CREATED_AT))

    @property
    def completed_at(self) -> DateFilter:
        return self._date(_raw_field(RESULT_RAW_ENUM_FIELD_COMPLETED_AT))

    @property
    def result_id(self) -> StringFilter:
        return self._string(_raw_field(RESULT_RAW_ENUM_FIELD_RESULT_ID))

    @property
    def size(self) -> NumberFilter:
        return self._number(_raw_field(RESULT_RAW_ENUM_FIELD_SIZE))
