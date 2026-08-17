# C++ package

`libarmonik` — the C++ client and worker library generated from the `.proto` files, distributed as pre-built
DEB/RPM/tar.gz packages or built from source.

## Installation

Pre-built packages are the fastest path. Each format is split into two components:

- **Runtime package** (`libarmonik` for both DEB and RPM): the shared libraries (`.so`) only. This is all you need
  to run an application linked against the ArmoniK API.
- **Devel package** (`libarmonik-dev` on DEB, `libarmonik-devel` on RPM): headers, static archives, and CMake
  config/target files needed to build against the API. It depends on the runtime package.

If you only deploy applications that consume the ArmoniK API, installing the runtime package is enough. If you are
developing against the API (compiling code that includes its headers or links via `find_package`), install the
devel package as well.

### Debian/Ubuntu

```bash
# Runtime only
sudo dpkg -i libarmonik-*.deb

# Runtime and development files
sudo dpkg -i libarmonik-*.deb libarmonik-dev-*.deb
```

### RHEL/UBI

```bash
# Runtime only
sudo rpm -ivh libarmonik-*.rpm

# Runtime and development files
sudo rpm -ivh libarmonik-*.rpm libarmonik-devel-*.rpm
```

### tar.gz archive

A `tar.gz` archive is also available for systems where DEB/RPM packages aren't suitable. Unlike the DEB/RPM
packages, it is **not** split into runtime/devel components: the archive bundles the shared libraries, headers, and
CMake config files together.

```bash
sudo tar -xzf libarmonik-*.tar.gz -C /usr/local --strip-components=1
```

Packages are attached to [GitHub Releases](https://github.com/aneoconsulting/ArmoniK.Api/releases); build them
yourself with the scripts in `packages/cpp/tools/packaging` (`make-deb.sh`, `make-rpm.sh`, `make-tar.gz.sh`) if
needed — each builds a Docker image with the required build dependencies and copies the resulting package to the
current directory.

## Using it

```cmake
find_package(ArmoniK.Api.Client CONFIG REQUIRED)
target_link_libraries(my_app PRIVATE ArmoniK.Api.Client)
```

See the [Quickstart](../getting-started/quickstart.md) for a minimal call, and
[Authentication](../getting-started/authentication.md) for securing the channel. For method-level detail, browse
the [C++ API reference](../api-reference/cpp/index.rst).

## Building from source

### Linux, using Docker

1. Install Docker following the [official documentation](https://docs.docker.com/engine/install/).
2. From the repository root:

   ```bash
   cd packages/cpp/tools
   ./compile.sh
   ```

`compile.sh` builds (or reuses) a Docker image from `Dockerfile.ubuntu`, and compiles the project inside it. The
compiled binaries end up in the install directory.

### Windows

Requires PowerShell, Visual Studio 2022, and Git. The (future) build script is expected to install these
prerequisites locally under `tools/win64`, since they aren't covered by Visual Studio 2022 or its CMake plugins:
Chocolatey, gRPC 1.54.0 built from source, CMake, NASM.

```{warning}
A Windows compilation script is not yet available. This section is a placeholder — contributions are welcome.
```

### Worker image for deployment

The worker image is a Docker image built specifically to be deployed in the ArmoniK infrastructure.

```bash
cd packages/cpp/tools
./build-worker.sh
```

This sets the image tag, changes to the repository root to find the Protos, and builds the worker image from
`Dockerfile.worker`. Verify it with:

```bash
docker images | grep armonik-api-cpp
```
