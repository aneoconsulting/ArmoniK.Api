from typing import Dict

from grpc import Channel

from ..protogen.client.versions_service_pb2_grpc import VersionsStub
from ..protogen.common.versions_common_pb2 import (
    ListVersionsRequest,
    ListVersionsResponse,
)


class ArmoniKVersions:
    def __init__(self, grpc_channel: Channel):
        """Result service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = VersionsStub(grpc_channel)

    def list_versions(self) -> Dict[str, str]:
        """Get versions of ArmoniK components.

        Return:
            A dictionnary mapping each component to its version.
        """
        request = ListVersionsRequest()
        response: ListVersionsResponse = self._client.ListVersions(request)
        return {"core": response.core, "api": response.api}
