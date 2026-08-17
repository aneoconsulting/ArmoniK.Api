# C# packages

Generated from the `.proto` files plus hand-written extensions (channel creation, options, worker helpers).

## Installation

```bash
dotnet add package ArmoniK.Api.Client
```

Additional packages (`ArmoniK.Api.Common`, `ArmoniK.Api.Common.Channel`, `ArmoniK.Api.Core`, `ArmoniK.Api.Worker`)
are only needed if you're implementing a worker or need lower-level channel/option types directly — see
[Find your package](index.md) for links.

## Updating

C# packages are published to NuGet automatically by CI on every release, and are also available on an edge
channel built from every commit to `main` — see [Releases](../releases/index.md).

## Using it

See the [Quickstart](../getting-started/quickstart.md) for a minimal call, and
[Authentication](../getting-started/authentication.md) for securing the channel with TLS/mTLS.

## Namespaces

| Namespace | Contents |
|---|---|
| `ArmoniK.Api.Client.Options` | Options classes to configure the client connection to the control plane. |
| `ArmoniK.Api.Client.Submitter` | `GrpcChannelFactory` and other utilities for connecting to the control plane, plus the generated gRPC client classes. |
| `ArmoniK.Api.Common.Channel.Utils`, `ArmoniK.Api.Common.Options` | Classes to create and configure gRPC channels between workers and polling agents. |
| `ArmoniK.Api.Core` | Generated gRPC classes used by [ArmoniK.Core](https://github.com/aneoconsulting/ArmoniK.Core). |
| `ArmoniK.Api.Common.Utils` | Helpers widely used across ArmoniK. |
| `ArmoniK.Api.Worker.Worker`, `ArmoniK.Api.Worker.Utils` | Helpers to implement a .NET worker, plus its generated gRPC classes. |
| `ArmoniK.Api.Worker.Tests` | Test classes for the worker. |

For method-level detail, browse the [C# API reference](../api-reference/csharp/index.rst).
