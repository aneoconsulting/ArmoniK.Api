from .conftest import all_rpc_called, rpc_called, get_client
from armonik.client import ArmoniKHealthChecks
from armonik.common import ServiceHealthCheckStatus


class TestArmoniKHealthChecks:
    def test_check_health(self):
        health_checks_client: ArmoniKHealthChecks = get_client("HealthChecks")
        services_health = health_checks_client.check_health()

        assert rpc_called("HealthChecks", "CheckHealth")
        assert services_health == {
            "mock": {
                "message": "Mock is healthy",
                "status": ServiceHealthCheckStatus.HEALTHY,
            }
        }

    def test_service_fully_implemented(self):
        assert all_rpc_called("HealthChecks")
