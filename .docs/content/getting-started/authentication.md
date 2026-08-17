# Authentication

ArmoniK.Api has no API keys or bearer tokens. The client authenticates to the control plane at the transport level,
through the gRPC channel's credentials:

| Mode | When | What you need |
|---|---|---|
| Insecure (plaintext) | Local development, trusted network | Just an endpoint |
| TLS | Encrypt traffic, verify the server | A CA certificate (`ca.crt`) |
| mTLS | Also authenticate the client to the server | CA certificate + client certificate + client key |

Which modes are actually available depends on how the ArmoniK control plane was deployed. See
[How to configure authentication in ArmoniK](https://armonik.readthedocs.io/en/latest/content/user-guide/how-to-configure-authentication.html)
(ArmoniK docs) for the deployment side, including where `ca.crt`, `client.submitter.crt` and
`client.submitter.key` come from.

All languages read the same configuration keys when driven by environment variables or configuration files — see
[Environment Variables](../how-to/envars/index.rst) for the full list (`GrpcClient__Endpoint`, `GrpcClient__CaCert`,
`GrpcClient__CertPem`, `GrpcClient__KeyPem`, `GrpcClient__AllowUnsafeConnection`, …).

## Python

`armonik.common.create_channel` picks insecure vs. secure from the URI scheme and loads certificates for you:

```python
from armonik.common import create_channel

# Insecure
channel = create_channel("armonik.local:5001")

# TLS
channel = create_channel("https://armonik.local:5001", certificate_authority="ca.crt")

# mTLS
channel = create_channel(
    "https://armonik.local:5001",
    certificate_authority="ca.crt",
    client_certificate="client.submitter.crt",
    client_key="client.submitter.key",
)
```

See [Connecting to ArmoniK securely using gRPC in Python](../how-to/grpc-secure-python.md) for the certificate CN /
`/etc/hosts` gotcha that comes up with ArmoniK's default self-signed certificates.

## C#

`GrpcChannelFactory.CreateChannel` builds a `GrpcChannel` from a `GrpcClient` options object:

```csharp
using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;

// Insecure
var channel = GrpcChannelFactory.CreateChannel(new GrpcClient { Endpoint = "http://armonik.local:5001" });

// TLS
var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
{
  Endpoint = "https://armonik.local:5001",
  CaCert   = "ca.crt",
});

// mTLS
var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
{
  Endpoint = "https://armonik.local:5001",
  CaCert   = "ca.crt",
  CertPem  = "client.submitter.crt",
  KeyPem   = "client.submitter.key",
});
```

If the certificate's Common Name doesn't match the endpoint hostname (ArmoniK's default certificates), set
`OverrideTargetName` to the CN, or add the CN to `/etc/hosts` and use it as the endpoint instead.

## C++

`ChannelFactory` builds the channel from a `Configuration` populated with the same `GrpcClient__*` keys used by C#
(declared in `armonik::api::common::options::ControlPlane`):

```cpp
#include "channel/ChannelFactory.h"
#include "options/ControlPlane.h"

using armonik::api::common::options::ControlPlane;

armonik::api::common::utils::Configuration config;
config.set(ControlPlane::EndpointKey, "https://armonik.local:5001");
config.set(ControlPlane::CaCertKey, "ca.crt");
// For mTLS, also set ControlPlane::UserCertKey and ControlPlane::UserKeyKey

armonik::api::client::ChannelFactory channel_factory(config, logger);
auto channel = channel_factory.create_channel();
```

## Angular / TypeScript

The generated services take a configured gRPC-web transport, so TLS is handled by whatever reverse proxy or gateway
terminates it in front of the browser — see
[Use ArmoniK API in an Angular App](../how-to/angular-integration.md) for a worked example.
