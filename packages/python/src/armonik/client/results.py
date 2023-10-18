from __future__ import annotations
from grpc import Channel

from typing import List, Dict, cast, Tuple

from ..protogen.client.results_service_pb2_grpc import ResultsStub
from ..protogen.common.results_common_pb2 import CreateResultsMetaDataRequest, CreateResultsMetaDataResponse, ListResultsRequest, ListResultsResponse
from ..protogen.common.results_filters_pb2 import Filters as rawFilters, FiltersAnd as rawFilterAnd, FilterField as rawFilterField, FilterStatus as rawFilterStatus
from ..protogen.common.results_fields_pb2 import ResultField
from ..common.filter import StringFilter, StatusFilter, DateFilter, NumberFilter, Filter
from ..protogen.common.sort_direction_pb2 import SortDirection
from ..common import Direction , Result
from ..protogen.common.results_fields_pb2 import ResultField, ResultRawField, ResultRawEnumField, RESULT_RAW_ENUM_FIELD_STATUS

class ResultFieldFilter:
    STATUS = StatusFilter(ResultField(result_raw_field=ResultRawField(field=RESULT_RAW_ENUM_FIELD_STATUS)), rawFilters, rawFilterAnd, rawFilterField, rawFilterStatus)

class ArmoniKResult:
    def __init__(self, grpc_channel: Channel):
        """ Result service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = ResultsStub(grpc_channel)

    def get_results_ids(self, session_id: str, names: List[str]) -> Dict[str, str]:
        return {r.name : r.result_id for r in cast(CreateResultsMetaDataResponse, self._client.CreateResultsMetaData(CreateResultsMetaDataRequest(results=[CreateResultsMetaDataRequest.ResultCreate(name = n) for n in names], session_id=session_id))).results}

    def list_results(self, result_filter: Filter, page: int = 0, page_size: int = 1000, sort_field: Filter = ResultFieldFilter.STATUS,sort_direction: SortDirection = Direction.ASC ) -> Tuple[int, List[Result]]:
        """List results based on a filter.

        Args:
            result_filter (Filter): Filter to apply when listing results
            page: page number to request, useful for pagination, defaults to 0
            page_size: size of a page, defaults to 1000
            sort_field: field to sort the resulting list by, defaults to the status
            sort_direction: direction of the sort, defaults to ascending
        Returns:
            A tuple containing :
            - The total number of results for the given filter
            - The obtained list of results
        """
        request: ListResultsRequest = ListResultsRequest(
            page=page,
            page_size=page_size,
            filters=cast(rawFilters, result_filter.to_disjunction().to_message()),
            sort=ListResultsRequest.Sort(field=cast(ResultField, sort_field.field), direction=sort_direction),
        )
        list_response: ListResultsResponse = self._client.ListResults(request)
        return list_response.total, [Result.from_message(r) for r in list_response.results]
