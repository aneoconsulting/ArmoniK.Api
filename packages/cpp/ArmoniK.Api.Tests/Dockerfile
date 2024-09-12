# Start with the latest Alpine base image for the build stage
FROM alpine:3.18.6 AS builder

# Install all the necessary dependencies required for the build process
# These include tools and libraries for building and compiling the source code
RUN apk update && apk add --no-cache \
    git \
    gcc \
    g++ \
    build-base \
    libtool \
    curl \
    c-ares \
    c-ares-dev \
    make \
    cmake \
    linux-headers \
    grpc \
    grpc-dev \
    protobuf \
    protobuf-dev \
    curl-dev

# Set environment variables for protobuf
ENV protobuf_BUILD_TESTS=OFF

# Update PATH with the new directories
ENV PATH="/app/install/lib:$PATH"
ENV PATH="/app/install/bin:$PATH"

# Print the PATH variable
RUN echo $PATH

# Copy the application source files into the image
WORKDIR /app/source
COPY packages/cpp/tools/packaging/common/. ./tools/packaging/common/
COPY Protos/V1/. ./Protos/
COPY packages/cpp/ArmoniK.Api.Common/. ./ArmoniK.Api.Common/
COPY packages/cpp/ArmoniK.Api.Client/. ./ArmoniK.Api.Client/
COPY packages/cpp/ArmoniK.Api.Tests/. ./ArmoniK.Api.Tests/
COPY packages/cpp/CMakeLists.txt .
COPY packages/cpp/Packaging.cmake .
COPY packages/cpp/Dependencies.cmake .
COPY packages/cpp/certs ./certs/

WORKDIR /app/build
RUN cmake -DCMAKE_BUILD_TYPE=Debug -DPROTO_FILES_DIR=/app/source/Protos -DCMAKE_INSTALL_PREFIX=/app/install -DBUILD_WORKER=OFF -DBUILD_TEST=ON /app/source/ && make -j $(nproc) install
# Set the default command to build the client using CMake and make
# CMD ["sh", "-c", "cmake -DCMAKE_BUILD_TYPE=Debug -DPROTO_FILES_DIR=/app/proto -DCMAKE_INSTALL_PREFIX=/app/install -DBUILD_TEST=OFF /app/source/ -DCPACK_GENERATOR=TGZ && make -j $(nproc) install && make package -j"]

# Uncomment the line below if you want the container to start in the bash shell by default
ENTRYPOINT ["/app/install/bin/ArmoniK.Api.Tests"]
