from typing import Any

from .filter import FilterWrapper, StringFilter
from ...protogen.common.applications_fields_pb2 import (
    ApplicationField,
    ApplicationRawField,
    APPLICATION_RAW_ENUM_FIELD_NAMESPACE,
    APPLICATION_RAW_ENUM_FIELD_SERVICE,
    APPLICATION_RAW_ENUM_FIELD_VERSION,
    APPLICATION_RAW_ENUM_FIELD_NAME,
)
from ...protogen.common.applications_filters_pb2 import Filters, FiltersAnd, FilterField


def _raw_field(field: Any) -> ApplicationField:
    return ApplicationField(application_field=ApplicationRawField(field=field))


class ApplicationFilter(FilterWrapper):
    """
    Filter for applications
    """

    def __init__(self):
        super().__init__(Filters, FiltersAnd, FilterField)

    @property
    def name(self) -> StringFilter:
        return self._string(_raw_field(APPLICATION_RAW_ENUM_FIELD_NAME))

    @property
    def namespace(self) -> StringFilter:
        return self._string(_raw_field(APPLICATION_RAW_ENUM_FIELD_NAMESPACE))

    @property
    def service(self) -> StringFilter:
        return self._string(_raw_field(APPLICATION_RAW_ENUM_FIELD_SERVICE))

    @property
    def version(self) -> StringFilter:
        return self._string(_raw_field(APPLICATION_RAW_ENUM_FIELD_VERSION))
