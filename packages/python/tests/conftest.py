import base64
import os

import pytest
import requests

from armonik.client import (
    ArmoniKEvents,
    ArmoniKHealthChecks,
    ArmoniKPartitions,
    ArmoniKResults,
    ArmoniKSessions,
    ArmoniKTasks,
    ArmoniKVersions,
)
from armonik.common.channel import create_channel, _find_bundle_path, _load_certificates
from armonik.protogen.common.worker_common_pb2 import ProcessRequest
from armonik.protogen.worker.agent_service_pb2_grpc import AgentStub
from typing import List, Union, Dict, Any

from google.protobuf.json_format import MessageToJson

ca_cert = os.getenv("Grpc__CaCert") or os.getenv("GrpcClient__CaCert") or None
client_cert = os.getenv("Grpc__ClientCert") or os.getenv("GrpcClient__CertPem") or None
client_key = os.getenv("Grpc__ClientKey") or os.getenv("GrpcClient__KeyPem") or None
scheme = os.getenv("AK_SCHEME", "http")
# Mock server endpoints used for the tests.
grpc_endpoint = os.getenv("Grpc__Endpoint", scheme + "://localhost:5001")
http_endpoint = os.getenv("Http__Endpoint", scheme + "://localhost:5000")
calls_endpoint = http_endpoint + "/calls.json"
reset_endpoint = http_endpoint + "/reset"
healthcheck_endpoint = http_endpoint + "/worker/healthcheck"
process_endpoint = http_endpoint + "/worker/process"
data_folder = os.getcwd()

request_ca = ca_cert if ca_cert is not None else _find_bundle_path()
if client_cert is not None:
    _, request_cert, request_key = _load_certificates(request_ca, client_cert, client_key)
    cert_path, key_path = (
        os.path.join(data_folder, "cert.pem"),
        os.path.join(data_folder, "key.pem"),
    )
    with open(cert_path, "wb") as f:
        f.write(request_cert)
    with open(key_path, "wb") as f:
        f.write(request_key)

    request_certs = (cert_path, key_path)
else:
    request_certs = None


@pytest.fixture(scope="class", autouse=True)
def clean_up(request):
    """
    This fixture runs at the session scope and is automatically used before and after
    running all the tests. It set up and teardown the testing environments by:
        - creating dummy files before testing begins;
        - clear files after testing;
        - resets the mocking gRPC server counters to maintain a clean testing environment.

    Yields:
        None: This fixture is used as a context manager, and the test code runs between
        the 'yield' statement and the cleanup code.

    Raises:
        requests.exceptions.HTTPError: If an error occurs when attempting to reset
        the mocking gRPC server counters.
    """
    # Write dumm payload and data dependency to files for testing purposes
    with open(os.path.join(data_folder, "payload-id"), "wb") as f:
        f.write("payload".encode())
    with open(os.path.join(data_folder, "dd-id"), "wb") as f:
        f.write("dd".encode())

    # Run all the tests
    yield

    # Remove the temporary files created for testing
    os.remove(os.path.join(data_folder, "payload-id"))
    os.remove(os.path.join(data_folder, "dd-id"))
    if os.path.exists(os.path.join(data_folder, "result-id")):
        os.remove(os.path.join(data_folder, "result-id"))

    # Reset the mock server counters
    try:
        response = requests.post(reset_endpoint, verify=request_ca, cert=request_certs)
        response.raise_for_status()
        print("\nMock server resetted.")
    except requests.exceptions.HTTPError as e:
        print("An error occurred when resetting the server: " + str(e))


def get_client(
    client_name: str, endpoint: str = grpc_endpoint
) -> Union[
    AgentStub,
    ArmoniKEvents,
    ArmoniKHealthChecks,
    ArmoniKPartitions,
    ArmoniKResults,
    ArmoniKSessions,
    ArmoniKTasks,
    ArmoniKVersions,
]:
    """
    Get the ArmoniK client instance based on the specified service name.

    Args:
        client_name (str): The name of the ArmoniK client to retrieve.
        endpoint (str, optional): The gRPC server endpoint. Defaults to grpc_endpoint.

    Returns:
        Union[AgentStub, ArmoniKEvents, ArmoniKHealthChecks, ArmoniKPartitions, ArmoniKResults, ArmoniKSessions, ArmoniKTasks, ArmoniKVersions]
            An instance of the specified ArmoniK client.

    Raises:
        ValueError: If the specified service name is not recognized.

    Example:
        >>> result_service = get_client("Results")
        >>> submitter_service = get_client("Submitter", "custom_endpoint")
    """
    channel = create_channel(
        endpoint,
        certificate_authority=ca_cert,
        client_certificate=client_cert,
        client_key=client_key,
    ).__enter__()
    if client_name == "Agent":
        return AgentStub(channel)
    if client_name == "Events":
        return ArmoniKEvents(channel)
    if client_name == "HealthChecks":
        return ArmoniKHealthChecks(channel)
    if client_name == "Partitions":
        return ArmoniKPartitions(channel)
    if client_name == "Sessions":
        return ArmoniKSessions(channel)
    if client_name == "Tasks":
        return ArmoniKTasks(channel)
    if client_name == "Versions":
        return ArmoniKVersions(channel)
    if client_name == "Results":
        return ArmoniKResults(channel)
    raise ValueError("Unknown service name: " + str(client_name))


def rpc_called(
    service_name: str, rpc_name: str, n_calls: int = 1, endpoint: str = calls_endpoint
) -> bool:
    """Check if a remote procedure call (RPC) has been made a specified number of times.
    This function uses ArmoniK.Api.Mock. It just gets the '/calls.json' endpoint.

    Args:
        service_name (str): The name of the service providing the RPC.
        rpc_name (str): The name of the specific RPC to check for the number of calls.
        n_calls (int, optional): The expected number of times the RPC should have been called. Default is 1.
        endpoint (str, optional): The URL of the remote service providing RPC information. Default to
            calls_endpoint.

    Returns:
        bool: True if the specified RPC has been called the expected number of times, False otherwise.

    Raises:
        requests.exceptions.RequestException: If an error occurs when requesting ArmoniK.Api.Mock.

    Example:
    >>> rpc_called("http://localhost:5000/calls.json", "Versions", "ListVersionss", 0)
    True
    """
    response = requests.get(endpoint, verify=request_ca, cert=request_certs)
    response.raise_for_status()
    data = response.json()

    # Check if the RPC has been called n_calls times
    if data[service_name][rpc_name] == n_calls:
        return True
    return False


def all_rpc_called(
    service_name: str, missings: List[str] = [], endpoint: str = calls_endpoint
) -> bool:
    """
    Check if all remote procedure calls (RPCs) in a service have been made at least once.
    This function uses ArmoniK.Api.Mock. It just gets the '/calls.json' endpoint.

    Args:
        service_name (str): The name of the service containing the RPC information in the response.
        endpoint (str, optional): The URL of the remote service providing RPC information. Default is
            the value of calls_endpoint.
        missings (List[str], optional): A list of RPCs known to be not implemented. Default is an empty list.

    Returns:
        bool: True if all RPCs in the specified service have been called at least once, False otherwise.

    Raises:
        requests.exceptions.RequestException: If an error occurs when requesting ArmoniK.Api.Mock.

    Example:
    >>> all_rpc_called("http://localhost:5000/calls.json", "Versions")
    False
    """
    response = requests.get(endpoint, verify=request_ca, cert=request_certs)
    response.raise_for_status()
    data = response.json()

    missing_rpcs = []

    # Check if all RPCs in the service have been called at least once
    for rpc_name, rpc_num_calls in data[service_name].items():
        if rpc_num_calls == 0:
            missing_rpcs.append(rpc_name)
    if missing_rpcs:
        if missings == missing_rpcs:
            return True
        print(f"RPCs not implemented in {service_name} service: {missing_rpcs}.")
        return False
    return True


def call_me_with_healthcheck(
    endpoint: str = healthcheck_endpoint,
) -> Union[str, Dict[str, Any]]:
    """
    Call the worker for a health check.
    Args:
        endpoint: endpoint to call.

    Returns:
        The result of the call.
    """
    response = requests.post(endpoint, verify=request_ca, cert=request_certs)
    response.raise_for_status()
    if "json" in response.headers["content-type"]:
        return response.json()
    return response.text


def call_me_with_process(
    request: ProcessRequest, results: Dict[str, bytes], endpoint: str = process_endpoint
) -> Union[str, Dict[str, Any]]:
    """
    Call the worker for Process call.
    Args:
        request: Process request to send to the worker.
        results: Task results used as data dependencies and payload
        endpoint: endpoint to call.

    Returns:
        The result of the call.
    """
    response = requests.post(
        endpoint,
        verify=request_ca,
        cert=request_certs,
        json={
            "Request": MessageToJson(request),
            "Results": {k: base64.b64encode(v).decode("ascii") for k, v in results.items()},
            "ResultsEncoding": "Base64",
        },
    )
    response.raise_for_status()
    if "json" in response.headers["content-type"]:
        return response.json()
    return response.text
