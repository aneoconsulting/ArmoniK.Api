from typing import Any

from .filter import FilterWrapper, StringFilter, NumberFilter, ArrayFilter
from ...protogen.common.partitions_fields_pb2 import (
    PARTITION_RAW_ENUM_FIELD_ID,
    PARTITION_RAW_ENUM_FIELD_PRIORITY,
    PARTITION_RAW_ENUM_FIELD_PREEMPTION_PERCENTAGE,
    PARTITION_RAW_ENUM_FIELD_POD_RESERVED,
    PARTITION_RAW_ENUM_FIELD_PARENT_PARTITION_IDS,
    PARTITION_RAW_ENUM_FIELD_POD_MAX,
    PartitionField,
    PartitionRawField,
)
from ...protogen.common.partitions_filters_pb2 import Filters, FiltersAnd, FilterField


def _raw_field(field: Any) -> PartitionField:
    return PartitionField(partition_raw_field=PartitionRawField(field=field))


class PartitionFilter(FilterWrapper):
    """
    Filter for partitions
    """

    def __init__(self):
        super().__init__(Filters, FiltersAnd, FilterField)

    @property
    def id(self) -> StringFilter:
        return self._string(_raw_field(PARTITION_RAW_ENUM_FIELD_ID))

    @property
    def priority(self) -> NumberFilter:
        return self._number(_raw_field(PARTITION_RAW_ENUM_FIELD_PRIORITY))

    @property
    def preemption_percentage(self) -> NumberFilter:
        return self._number(_raw_field(PARTITION_RAW_ENUM_FIELD_PREEMPTION_PERCENTAGE))

    @property
    def pod_reserved(self) -> NumberFilter:
        return self._number(_raw_field(PARTITION_RAW_ENUM_FIELD_POD_RESERVED))

    @property
    def pod_max(self) -> NumberFilter:
        return self._number(_raw_field(PARTITION_RAW_ENUM_FIELD_POD_MAX))

    @property
    def parent_partition_ids(self) -> ArrayFilter:
        return self._array(_raw_field(PARTITION_RAW_ENUM_FIELD_PARENT_PARTITION_IDS))
