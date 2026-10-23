# What is ArmoniK.Api?

ArmoniK.Api defines the gRPC contract for [ArmoniK](https://github.com/aneoconsulting/ArmoniK), a distributed task
scheduler for high-throughput computing. It is the `.proto` files under
[`Protos/V1`](https://github.com/aneoconsulting/ArmoniK.Api/tree/main/Protos/V1), plus the client and worker libraries
generated from them for several languages.

## Who it's for

- **Client developers** who need to submit tasks to an ArmoniK cluster and retrieve their results — use one of the
  client packages (C#, Python, TypeScript, Angular, C++).
- **Worker developers** who implement the compute side that ArmoniK's control plane dispatches tasks to — use the C#
  or C++ worker packages.

Either way, ArmoniK.Api only gets you *talking* to ArmoniK. It does not run a cluster: you need a reachable
**ArmoniK control plane** to connect to. See the [ArmoniK documentation](https://armonik.readthedocs.io/en/latest/index.html)
for how to deploy one.

## Available packages

| Language | Package | Registry |
|---|---|---|
| C# | `ArmoniK.Api.Client`, `ArmoniK.Api.Worker`, … | NuGet |
| Python | `armonik` | PyPI |
| TypeScript | `@aneoconsultingfr/armonik.api` | npm |
| Angular | `@aneoconsultingfr/armonik.api.angular` | npm |
| C++ | `libarmonik` | GitHub Releases |

See [Find your package](../packages/index.md) for links to each registry.

## Next steps

- **Quickstart** — connect and make your first call in a few lines: [Quickstart](../getting-started/quickstart.md)
- **Authentication** — secure the connection with TLS/mTLS: [Authentication](../getting-started/authentication.md)
- **Concepts** — Session, Task, Result, Partition and how they fit together: [Glossary](../concepts/glossary.md)
- **API reference** — browse the generated API docs for each language: [API Reference](../api-reference/overview.md)
- **How-to guides** — task-oriented guides, including C++ compilation and Angular integration: [How-to guides](../how-to/index.md)
- **Releases** — versioning scheme and how to cut a new release: [Releases](../releases/index.md)
