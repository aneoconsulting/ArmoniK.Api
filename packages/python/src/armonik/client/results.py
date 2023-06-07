from grpc import Channel

from typing import List, Dict, cast

from ..protogen.client.results_service_pb2_grpc import ResultsStub
from ..protogen.common.results_common_pb2 import CreateResultsMetaDataRequest, CreateResultsMetaDataResponse


class ArmoniKResult:
    def __init__(self, grpc_channel: Channel):
        """ Result service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = ResultsStub(grpc_channel)

    def get_results_ids(self, session_id: str, names : List[str]) -> Dict[str, str]:
        return {r.name : r.result_id for r in cast(CreateResultsMetaDataResponse, self._client.CreateResultsMetaData(CreateResultsMetaDataRequest(results=[CreateResultsMetaDataRequest.ResultCreate(name = n) for n in names], session_id=session_id))).results}