# Quickstart

```{important}
This requires a reachable ArmoniK control plane — ArmoniK.Api is a client library, it doesn't run one for you. If
you don't have a cluster yet, deploy one following the
[ArmoniK documentation](https://armonik.readthedocs.io/en/latest/index.html) first; the calls below then take
under 5 minutes.
```

`Versions.ListVersions` is the simplest method in the API: it takes no session, no task, no auth setup beyond a
plain channel. It's a good way to confirm your client can reach the control plane before building anything else.

The examples below assume the control plane is listening on `localhost:5001` without TLS. For a secured connection,
see [Authentication](authentication.md).

## Python

```bash
pip install armonik
```

```python
from armonik.client import ArmoniKVersions
from armonik.common import create_channel

channel = create_channel("localhost:5001")
versions = ArmoniKVersions(channel)
print(versions.list_versions())
# {'core': '3.19.0', 'api': '3.19.0'}
```

## C#

```bash
dotnet add package ArmoniK.Api.Client
```

```csharp
using ArmoniK.Api.gRPC.V1.Versions;
using Grpc.Net.Client;

var channel = GrpcChannel.ForAddress("http://localhost:5001");
var client = new Versions.VersionsClient(channel);
var response = client.ListVersions(new ListVersionsRequest());
Console.WriteLine($"core={response.Core}, api={response.Api}");
```

## C++

```cmake
find_package(ArmoniK.Api.Client CONFIG REQUIRED)
target_link_libraries(my_app PRIVATE ArmoniK.Api.Client)
```

```cpp
#include <grpcpp/grpcpp.h>
#include "versions/VersionsClient.h"

int main() {
  auto channel = grpc::CreateChannel("localhost:5001", grpc::InsecureChannelCredentials());
  armonik::api::client::VersionsClient client(armonik::api::grpc::v1::versions::Versions::NewStub(channel));

  auto versions = client.list_versions();
  std::cout << "core=" << versions.core << " api=" << versions.api << std::endl;
}
```

See [Compilation steps for cpp API](../packages/cpp.md) for how to install `libarmonik` before this will link.

## Next steps

- [Authentication](authentication.md) — secure the channel with TLS/mTLS for a real deployment.
- [Submit a task and retrieve its result](../concepts/submit-and-retrieve-results.md) — the core end-to-end workflow.
- [Glossary](../concepts/glossary.md) — Session, Task, Result, Partition and other terms used throughout the reference.
