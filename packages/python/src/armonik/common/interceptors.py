import grpc

from armonik.client import ArmoniKVersions
from grpc_interceptor import ClientCallDetails, ClientInterceptor
from typing import Callable, Any


class ArmoniKException(Exception):

    def __init__(self, service, rpc_name, error_message):
        self.service = service
        self.rpc_name = rpc_name
        self.message = error_message


class GrpcExceptionsInterceptor(ClientInterceptor):
    """Custom gRPC client interceptor to catch gRPC exceptions caused by a
    server error and return a suitable exception instead."""

    def intercept(
        self,
        method: Callable,
        request_or_iterator: Any,
        call_details: grpc.ClientCallDetails,
    ):
        """This method is called for all unary and streaming RPCs. The interceptor
        implementation should call `method` using a `grpc.ClientCallDetails` and the
        `request_or_iterator` object as parameters. The `request_or_iterator`
        parameter may be type checked to determine if this is a singluar request
        for unary RPCs or an iterator for client-streaming or client-server streaming
        RPCs.

        Args:
            method: A function that proceeds with the invocation by executing the next
                interceptor in the chain or invoking the actual RPC on the underlying
                channel.
            request_or_iterator: RPC request message or iterator of request messages
                for streaming requests.
            call_details: Describes an RPC to be invoked.

        Returns:
            The type of the return should match the type of the return value received
            by calling `method`. This is an object that is both a
            `Call <https://grpc.github.io/grpc/python/grpc.html#grpc.Call>`_ for the
            RPC and a `Future <https://grpc.github.io/grpc/python/grpc.html#grpc.Future>`_.
        """
        future = method(request_or_iterator, call_details)
        if isinstance(future, grpc.RpcError):
            service = call_details.method.split("/")[1].split(".")[-1]
            rpc_name = call_details.method.split("/")[-1]
            match future.code():
                case grpc.StatusCode.UNAVAILABLE:
                    raise ArmoniKException(service, rpc_name, future.debug_error_string())
        return future
