from grpc import Channel

from ..protogen.client.health_checks_service_pb2_grpc import HealthChecksServiceStub
from ..protogen.common.health_checks_common_pb2 import (
    CheckHealthRequest,
    CheckHealthResponse,
)


class ArmoniKHealthChecks:
    def __init__(self, grpc_channel: Channel):
        """Result service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = HealthChecksServiceStub(grpc_channel)

    def check_health(self):
        response: CheckHealthResponse = self._client.CheckHealth(CheckHealthRequest())
        return {
            service.name: {"message": service.message, "status": service.healthy}
            for service in response.services
        }
