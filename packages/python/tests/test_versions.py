import json

from .conftest import all_rpc_called, rpc_called, get_client
from armonik.client import ArmoniKVersions


class TestArmoniKVersions:
    def test_list_versions(self):
        """
        Test the list_versions method of ArmoniKVersions client.

        Args:
            grpc_endpoint (str): The gRPC endpoint for the service mock.
            calls_recap_endpoint (str): The endpoint for tracking RPC calls.

        Assertions:
            Ensures that the RPC 'ListVersions' is called on the service 'Versions'.
            Asserts that the 'core' version is returned with correct value.
            Asserts that the 'api' version is returned with correct value.
        """
        versions_client: ArmoniKVersions = get_client("Versions")
        versions = versions_client.list_versions()

        assert rpc_called("Versions", "ListVersions")
        assert versions["core"] == "Unknown"
        with open("../web/package.json", "r") as file:
            assert versions["api"] == json.loads(file.read())["version"] + ".0"

    def test_service_fully_implemented(self):
        """
        Test if all RPCs in the 'Versions' service have been called at least once.

        Args:
            calls_recap_endpoint (str): The endpoint for tracking RPC calls.

        Assertions:
            Ensures that all RPCs in the 'Versions' service have been called at least once.
        """
        assert all_rpc_called("Versions")
