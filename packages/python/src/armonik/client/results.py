from __future__ import annotations

from typing import Dict, List, Tuple, cast, Optional, Union

from deprecation import deprecated
from grpc import Channel

from .. import __version__
from ..common import Direction, Result
from ..common.filter import Filter, ResultFilter
from ..common.helpers import batched
from ..protogen.client.results_service_pb2_grpc import ResultsStub
from ..protogen.common.objects_pb2 import Empty
from ..protogen.common.results_common_pb2 import (
    CreateResultsMetaDataRequest,
    CreateResultsMetaDataResponse,
    CreateResultsRequest,
    CreateResultsResponse,
    DeleteResultsDataRequest,
    DownloadResultDataRequest,
    GetOwnerTaskIdRequest,
    GetOwnerTaskIdResponse,
    GetResultRequest,
    GetResultResponse,
    ListResultsRequest,
    ListResultsResponse,
    ResultsServiceConfigurationResponse,
    UploadResultDataRequest,
)
from ..protogen.common.results_fields_pb2 import (
    ResultField,
)
from ..protogen.common.sort_direction_pb2 import SortDirection


@deprecated("3.19.0", None, __version__, "Use Result.<name of the field> instead")
class ResultFieldFilter:
    STATUS = Result.status
    RESULT_ID = Result.result_id


class ArmoniKResults:
    def __init__(self, grpc_channel: Channel):
        """Result service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = ResultsStub(grpc_channel)

    @deprecated(
        deprecated_in="3.15.0",
        details="Use create_result_metadata or create_result insted.",
    )
    def get_results_ids(self, session_id: str, names: List[str]) -> Dict[str, str]:
        return {
            r.name: r.result_id
            for r in cast(
                CreateResultsMetaDataResponse,
                self._client.CreateResultsMetaData(
                    CreateResultsMetaDataRequest(
                        results=[CreateResultsMetaDataRequest.ResultCreate(name=n) for n in names],
                        session_id=session_id,
                    )
                ),
            ).results
        }

    def list_results(
        self,
        result_filter: Optional[Filter] = None,
        page: int = 0,
        page_size: int = 1000,
        sort_field: Filter = Result.status,
        sort_direction: SortDirection = Direction.ASC,
    ) -> Tuple[int, List[Result]]:
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
            filters=(
                ResultFilter().to_message() if result_filter is None else result_filter.to_message()
            ),
            sort=ListResultsRequest.Sort(
                field=cast(ResultField, sort_field.field), direction=sort_direction
            ),
        )
        list_response: ListResultsResponse = self._client.ListResults(request)
        return list_response.total, [Result.from_message(r) for r in list_response.results]

    def get_result(self, result_id: str) -> Result:
        """Get a result by id.

        Args:
            result_id: The ID of the result.

        Return:
            The result summary.
        """
        request = GetResultRequest(result_id=result_id)
        response: GetResultResponse = self._client.GetResult(request)
        return Result.from_message(response.result)

    def get_owner_task_id(
        self, result_ids: List[str], session_id: str, batch_size: int = 500
    ) -> Dict[str, str]:
        """Get the IDs of the tasks that should produce the results.

        Args:
            result_ids: A list of results.
            session_id: The ID of the session to which the results belongs.
            batch_size: Batch size for querying.

        Return:
            A dictionnary mapping results to owner task ID.
        """
        results = {}
        for result_ids_batch in batched(result_ids, batch_size):
            request = GetOwnerTaskIdRequest(session_id=session_id, result_id=result_ids_batch)
            response: GetOwnerTaskIdResponse = self._client.GetOwnerTaskId(request)
            for result_task in response.result_task:
                results[result_task.result_id] = result_task.task_id
        return results

    def create_results_metadata(
        self, result_names: List[str], session_id: str, batch_size: int = 100
    ) -> Dict[str, Result]:
        """Create the metadata of multiple results at once.
        Data have to be uploaded separately.

        Args:
            result_names: The list of the names of the results to create.
            session_id: The ID of the session to which the results belongs.
            batch_size: Batch size for querying.

        Return:
            A dictionnary mapping each result name to its corresponding result summary.
        """
        results = {}
        for result_names_batch in batched(result_names, batch_size):
            request = CreateResultsMetaDataRequest(
                results=[
                    CreateResultsMetaDataRequest.ResultCreate(name=result_name)
                    for result_name in result_names_batch
                ],
                session_id=session_id,
            )
            response: CreateResultsMetaDataResponse = self._client.CreateResultsMetaData(request)
            for result_message in response.results:
                results[result_message.name] = Result.from_message(result_message)
        return results

    def create_results(
        self, results_data: Dict[str, bytes], session_id: str, batch_size: int = 1
    ) -> Dict[str, Result]:
        """Create one result with data included in the request.

        Args:
            results_data: A dictionnary mapping the result names to their actual data.
            session_id: The ID of the session to which the results belongs.
            batch_size: Batch size for querying.

        Return:
            A dictionnary mappin each result name to its corresponding result summary.
        """
        results = {}
        for results_names_batch in batched(results_data.keys(), batch_size):
            request = CreateResultsRequest(
                results=[
                    CreateResultsRequest.ResultCreate(name=name, data=results_data[name])
                    for name in results_names_batch
                ],
                session_id=session_id,
            )
            response: CreateResultsResponse = self._client.CreateResults(request)
            for message in response.results:
                results[message.name] = Result.from_message(message)
        return results

    def upload_result_data(
        self, result_id: str, session_id: str, result_data: Union[bytes, bytearray]
    ) -> None:
        """Upload data for an empty result already created.

        Args:
            result_id: The ID of the result.
            result_data: The result data.
            session_id: The ID of the session.
        """
        data_chunk_max_size = self.get_service_config()

        def upload_result_stream():
            request = UploadResultDataRequest(
                id=UploadResultDataRequest.ResultIdentifier(
                    session_id=session_id, result_id=result_id
                )
            )
            yield request

            start = 0
            data_len = len(result_data)
            while start < data_len:
                chunk_size = min(data_chunk_max_size, data_len - start)
                request = UploadResultDataRequest(
                    data_chunk=result_data[start : start + chunk_size]
                )
                yield request
                start += chunk_size

        self._client.UploadResultData(upload_result_stream())

    def download_result_data(self, result_id: str, session_id: str) -> bytes:
        """Retrieve data of a result.

        Args:
            result_id: The ID of the result.
            session_id: The session of the result.

        Return:
            Result data.
        """
        request = DownloadResultDataRequest(result_id=result_id, session_id=session_id)
        streaming_call = self._client.DownloadResultData(request)
        return b"".join([message.data_chunk for message in streaming_call])

    def delete_result_data(
        self, result_ids: List[str], session_id: str, batch_size: int = 100
    ) -> None:
        """Delete data from multiple results

        Args:
            result_ids: The IDs of the results which data must be deleted.
            session_id: The ID of the session to which the results belongs.
            batch_size: Batch size for querying.
        """
        for result_ids_batch in batched(result_ids, batch_size):
            request = DeleteResultsDataRequest(result_id=result_ids_batch, session_id=session_id)
            self._client.DeleteResultsData(request)

    def get_service_config(self) -> int:
        """Get the configuration of the service.

        Return:
            Maximum size supported by a data chunk for the result service.
        """
        response: ResultsServiceConfigurationResponse = self._client.GetServiceConfiguration(
            Empty()
        )
        return response.data_chunk_max_size

    def watch_results(self):
        raise NotImplementedError()
