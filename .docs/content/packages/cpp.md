# Compilation steps for cpp API

## Compilation of ArmoniK.Api.cpp Client and Server on Linux using Docker

In order to compile the client and server on Linux, we use a Docker image to set up the necessary environment and dependencies. This ensures a consistent and clean environment for compilation.

### Prerequisites Linux

1. Install Docker on your Linux system. Follow the instructions on the [official Docker documentation](https://docs.docker.com/engine/install/).
2. Clone the repository containing the source code and the necessary scripts.

### Compilation Steps for Linux

1. Open a terminal in the root directory of the cloned repository.
2. Run the `compile.sh` script. It compiles the C++ project on Linux systems:

```bash
cd packages/cpp/tools
./compile.sh
```

The `compile.sh` script does the following:

- Sets the image tag for the Docker image.
- Determines the absolute paths of the necessary directories (working, proto, build, and install directories).
- Checks if the Docker image exists. If not, it builds the Docker image using the Dockerfile.ubuntu file.
- Compiles the project source using the Docker image.
Once the compilation is complete, the compiled binaries will be located in the install directory.

Now you have successfully compiled the client and server on Linux using Docker.

### Compiling the Client and Server on Windows

### Prerequisites Windows

Before getting started, make sure you have the following tools and packages installed on your machine:

- PowerShell
- Visual Studio 2022
- Git

The (future) build script is expected to install the following prerequisites locally, under `tools/win64`, since they aren't covered by Visual Studio 2022 or its CMake plugins:

- Chocolatey package manager
- gRPC 1.54.0 built from source
- CMake
- NASM

```{warning}
A Windows compilation script is not yet available. This section is a placeholder — contributions are welcome.
```

## Compilation of the Worker ArmoniK.Api.cpp Image for Deployment in ArmoniK Infrastructure

The worker image is a Docker image that is built specifically to be deployed in the ArmoniK infrastructure. This image contains the necessary dependencies and configurations for the worker to function correctly.

### Prerequisites

1. Install Docker on your Linux system. Follow the instructions on the [official Docker documentation](https://docs.docker.com/engine/install/).
2. Clone the repository containing the source code and the necessary scripts.

### Compilation Steps

1. Open a terminal in the root directory of the cloned repository.
2. Run the `build-worker.sh` script:

   ```bash
   cd packages/cpp/tools
   ./build-worker.sh
   ```

   The build-worker.sh script does the following:

   - Sets the image tag for the Docker image.
   - Determines the absolute paths of the necessary directories (script, working, and root directories).
   - Changes to the root directory where the Protos are located.
   - Builds the worker Docker image using the Dockerfile.worker file.

   Now you should have the final image

3. Once the worker image has been built, you can use the following command to list all the Docker images available on your system:

   ```bash
   docker images | grep armonik-api-cpp
   ```

The worker image should be listed with the specified image tag (e.g., armonik-api-cpp:v0.1).

Now you have successfully compiled the worker image for deployment in the ArmoniK infrastructure.

## Installing Pre-built DEB/RPM Packages

Instead of compiling from source, you can install the ArmoniK C++ API from pre-built DEB (Debian/Ubuntu) or RPM (RHEL/UBI) packages. Each format is split into two components:

- **Runtime package** (`libarmonik` for both DEB and RPM): the shared libraries (`.so`) only. This is all you need to run an application linked against the ArmoniK API.
- **Devel package** (`libarmonik-dev` on DEB, `libarmonik-devel` on RPM): headers, static archives, and CMake config/target files needed to build against the API. It depends on the runtime package.

If you only deploy applications that consume the ArmoniK API, installing the runtime package is enough. If you are developing against the API (compiling code that includes its headers or links via `find_package`), install the devel package as well.

### Building the packages

```bash
cd packages/cpp/tools/packaging
./make-deb.sh    # produces libarmonik and libarmonik-dev .deb files
./make-rpm.sh    # produces libarmonik and libarmonik-devel .rpm files
```

Each script builds a Docker image with the required build dependencies, compiles the project with the corresponding CPack generator (`DEB` or `RPM`), and copies the resulting packages to the current directory.

### Installing on Debian/Ubuntu

```bash
# Runtime only
sudo dpkg -i libarmonik-*.deb

# Runtime and development files
sudo dpkg -i libarmonik-*.deb libarmonik-dev-*.deb
```

### Installing on RHEL/UBI

```bash
# Runtime only
sudo rpm -ivh libarmonik-*.rpm

# Runtime and development files
sudo rpm -ivh libarmonik-*.rpm libarmonik-devel-*.rpm
```

### Building and installing a tar.gz archive

A `tar.gz` archive is also available for systems where DEB/RPM packages aren't suitable. Unlike the DEB/RPM packages, it is **not** split into runtime/devel components: the archive bundles the shared libraries, headers, and CMake config files together.

```bash
cd packages/cpp/tools/packaging
./make-tar.gz.sh    # produces a libarmonik-*.tar.gz archive
```

Install it by extracting it to the desired prefix (e.g. `/usr/local`):

```bash
sudo tar -xzf libarmonik-*.tar.gz -C /usr/local --strip-components=1
```
