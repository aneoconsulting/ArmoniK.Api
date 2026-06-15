# Environment Variables exposed in ArmoniK.Api
## Options for GrpcClient

- **GrpcClient__Endpoint**: string? (default: `null`)

    Endpoint for sending requests

- **GrpcClient__AllowUnsafeConnection**: bool (default: `false`)

    Allow unsafe connections to the endpoint (without SSL), defaults to false

- **GrpcClient__CertPem**: string (default: `""`)

    Path to the certificate file in pem format

- **GrpcClient__KeyPem**: string (default: `""`)

    Path to the key file in pem format

- **GrpcClient__CertP12**: string (default: `""`)

    Path to the certificate file in p12/pfx format

- **GrpcClient__CaCert**: string (default: `""`)

    Path to the Certificate Authority file in pem format

- **GrpcClient__OverrideTargetName**: string (default: `""`)

    Override the target name used during TLS SNI and certificate CN verification. Use this when the certificate's Common Name (CN) does not match the endpoint hostname (e.g. ArmoniK's default self-signed certificates).
Defaults to the endpoint hostname. Only applies to .NET Framework; on .NET Core the SNI is set automatically.

- **GrpcClient__HasClientCertificate**: bool (default: `false`)

    True if the options specify a client certificate

- **GrpcClient__KeepAliveTime**: TimeSpan (default: `TimeSpan.FromSeconds(30)`)

    KeepAliveTime is the time after which the connection will be kept alive.

- **GrpcClient__KeepAliveTimeInterval**: TimeSpan (default: `TimeSpan.FromSeconds(30)`)

    KeepAliveTimeInterval is the interval at which the connection will be kept alive.

- **GrpcClient__MaxIdleTime**: TimeSpan (default: `TimeSpan.FromMinutes(5)`)

    MaxIdleTime is the maximum idle time after which the connection will be closed.

- **GrpcClient__MaxAttempts**: int (default: `5`)

    MaxAttempts is a property that gets and sets the maximum number of attempts to retry an operation.

- **GrpcClient__BackoffMultiplier**: double (default: `1.5`)

    The backoff will be multiplied by this multiplier after each retry attempt and will increase exponentially when the
multiplier is greater than 1.

- **GrpcClient__InitialBackOff**: TimeSpan (default: `TimeSpan.FromSeconds(1)`)

    InitialBackOff is a property that gets and sets the initial backOff time for retrying an operation.

- **GrpcClient__MaxBackOff**: TimeSpan (default: `TimeSpan.FromSeconds(5)`)

    MaxBackOff is a property that gets and sets the maximum backOff time for retrying an operation.

- **GrpcClient__RequestTimeout**: TimeSpan (default: `Timeout.InfiniteTimeSpan`)

    Timeout for grpc requests. Defaults to no timeout.

- **GrpcClient__HttpMessageHandler**: string (default: `""`)

    Which HttpMessageHandler to use.
Valid options:
- `HttpClientHandler`
- `WinHttpHandler`
- `GrpcWebHandler`
If the handler is not set, the best one will be used.

- **GrpcClient__Proxy**: string (default: `""`)

    Proxy configuration.
If empty, the default proxy configuration is used.
If "none", proxy is disabled.
If "system", the system proxy is used
Otherwise, it is the URL of the proxy to use

- **GrpcClient__ProxyUsername**: string (default: `""`)

    Username used for proxy authentication

- **GrpcClient__ProxyPassword**: string (default: `""`)

    Password used for proxy authentication

- **GrpcClient__ReusePorts**: bool (default: `true`)

    Enable the option SO_REUSE_UNICASTPORT upon socket opening to limit port exhaustion

## Options for ComputePlane

- **ComputePlane__[WorkerChannel](#options-for-grpcchannel)__Address**: string (default: `"/tmp/armonik.sock"`)

    Address or path of the resource used to communicate for this gRPC Channel

- **ComputePlane__[WorkerChannel](#options-for-grpcchannel)__KeepAlivePingTimeOut**: TimeSpan (default: `null`)

    Keep-alive ping timeout for http2 connections

- **ComputePlane__[WorkerChannel](#options-for-grpcchannel)__KeepAliveTimeOut**: TimeSpan (default: `null`)

    Keep-alive timeout

- **ComputePlane__[AgentChannel](#options-for-grpcchannel)__Address**: string (default: `"/tmp/armonik.sock"`)

    Address or path of the resource used to communicate for this gRPC Channel

- **ComputePlane__[AgentChannel](#options-for-grpcchannel)__KeepAlivePingTimeOut**: TimeSpan (default: `null`)

    Keep-alive ping timeout for http2 connections

- **ComputePlane__[AgentChannel](#options-for-grpcchannel)__KeepAliveTimeOut**: TimeSpan (default: `null`)

    Keep-alive timeout

- **ComputePlane__MessageBatchSize**: int (default: `1`)

    Number of messages retrieved from the queue by the Agent

- **ComputePlane__AbortAfter**: TimeSpan (default: `TimeSpan.Zero`)

    Time to wait upon cancellation before aborting the worker

## Options for GrpcChannel

- **GrpcChannel__Address**: string (default: `"/tmp/armonik.sock"`)

    Address or path of the resource used to communicate for this gRPC Channel

- **GrpcChannel__KeepAlivePingTimeOut**: TimeSpan (default: `null`)

    Keep-alive ping timeout for http2 connections

- **GrpcChannel__KeepAliveTimeOut**: TimeSpan (default: `null`)

    Keep-alive timeout

## Grpc Socket type
- **Tcp**
    Tcp Connection
- **UnixDomainSocket**
    Unix socket connection

