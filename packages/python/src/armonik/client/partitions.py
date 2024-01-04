from typing import cast, List, Tuple

from grpc import Channel

from ..common import Direction, Partition
from ..common.filter import Filter, NumberFilter
from ..protogen.client.partitions_service_pb2_grpc import PartitionsStub
from ..protogen.common.partitions_common_pb2 import ListPartitionsRequest, ListPartitionsResponse, GetPartitionRequest, GetPartitionResponse
from ..protogen.common.partitions_fields_pb2 import PartitionField, PartitionRawField, PARTITION_RAW_ENUM_FIELD_PRIORITY
from ..protogen.common.partitions_filters_pb2 import Filters as rawFilters, FiltersAnd as rawFiltersAnd, FilterField as rawFilterField
from ..protogen.common.sort_direction_pb2 import SortDirection


class PartitionFieldFilter:
    PRIORITY = NumberFilter(
        PartitionField(partition_raw_field=PartitionRawField(field=PARTITION_RAW_ENUM_FIELD_PRIORITY)),
        rawFilters,
        rawFiltersAnd,
        rawFilterField
    )


class ArmoniKPartitions:
    def __init__(self, grpc_channel: Channel):
        """ Result service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = PartitionsStub(grpc_channel)

    def list_partitions(self, partition_filter: Filter | None = None, page: int = 0, page_size: int = 1000, sort_field: Filter = PartitionFieldFilter.PRIORITY, sort_direction: SortDirection = Direction.ASC) -> Tuple[int, List[Partition]]:
        """List partitions based on a filter.

        Args:
            partition_filter: Filter to apply when listing partitions
            page: page number to request, useful for pagination, defaults to 0
            page_size: size of a page, defaults to 1000
            sort_field: field to sort the resulting list by, defaults to the status
            sort_direction: direction of the sort, defaults to ascending

        Returns:
            A tuple containing :
            - The total number of results for the given filter
            - The obtained list of results
        """
        request = ListPartitionsRequest(
            page=page,
            page_size=page_size,
            filters=cast(rawFilters, partition_filter.to_disjunction().to_message()) if partition_filter else None,
            sort=ListPartitionsRequest.Sort(field=cast(PartitionField, sort_field.field), direction=sort_direction),
        )
        response: ListPartitionsResponse = self._client.ListPartitions(request)
        return response.total, [Partition.from_message(p) for p in response.partitions]

    def get_partition(self, partition_id: str) -> Partition:
        """Get a partition by its ID.
        
        Args:
            partition_id: The partition ID.
        
        Return:
            The partition summary.
        """
        return Partition.from_message(self._client.GetPartition(GetPartitionRequest(id=partition_id)).partition)
