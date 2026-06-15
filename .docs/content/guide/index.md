# Get started

ArmoniK.Api is the interface layer for the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) distributed task scheduler. It defines the gRPC protocol between clients, workers, and the control plane, and provides generated client and worker libraries for multiple languages.

## Available packages

| Language | Package | Registry |
|---|---|---|
| C# | `ArmoniK.Api.Client`, `ArmoniK.Api.Worker`, … | NuGet |
| Python | `armonik` | PyPI |
| TypeScript | `@aneoconsultingfr/armonik.api` | npm |
| Angular | `@aneoconsultingfr/armonik.api.angular` | npm |
| C++ | `libarmonik` | GitHub Releases |

See [Find your package](packages/index.md) for links to each registry.

## Next steps

- **Releases** — learn how versioning works and how to create a new release: [Releases](releases.md)
- **C++ compilation** — build the C++ client and worker locally: [Compilation steps for C++](cpp.md)
- **Usage guides** — connect securely, integrate with Angular, configure environment variables: [Usage](../usage/use-armonik-grpc-securely-python.md)
- **API reference** — browse the generated API docs for each language: [API](../api/index.md)
