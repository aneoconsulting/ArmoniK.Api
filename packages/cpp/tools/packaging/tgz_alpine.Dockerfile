# Start with the latest Alpine base image for the build stage
FROM alpine AS builder
ARG GRPC_VERSION=v1.54.0

# Install all the necessary dependencies required for the build process
# These include tools and libraries for building and compiling the source code
RUN apk update && apk add --no-cache \
    git \
    gcc \
    g++ \
    build-base \
    autoconf \
    automake \
    libtool \
    curl \
    c-ares \
    c-ares-dev \
    make \
    cmake \
    unzip \
    linux-headers \ 
    grpc \
    grpc-dev \
    protobuf \
    protobuf-dev

# Update the PATH environment variable to include the gRPC libraries and binaries
ENV LD_LIBRARY_PATH="/app/install/lib:$LD_LIBRARY_PATH"
ENV PATH="/app/install/bin:$PATH"

# Display the updated PATH environment variable
RUN echo $PATH

# Copy the application source files into the image
WORKDIR /app/libarmonik
COPY packages/cpp/tools/packaging/common/. ./tools/packaging/common/
COPY Protos/V1/. ./Protos/
COPY packages/cpp/ArmoniK.Api.Common/. ./ArmoniK.Api.Common/
COPY packages/cpp/ArmoniK.Api.Client/. ./ArmoniK.Api.Client/
COPY packages/cpp/ArmoniK.Api.Worker/. ./ArmoniK.Api.Worker/
COPY packages/cpp/CMakeLists.txt .
COPY packages/cpp/Packaging.cmake .
COPY packages/cpp/Dependencies.cmake .

# Build the application using the copied source files and protobuf definitions
WORKDIR /app/libarmonik/build
RUN cmake -DCMAKE_INSTALL_PREFIX="/app/install" -DPROTO_FILES_DIR="/app/proto" -DBUILD_TEST=OFF \
          -DBUILD_CLIENT=ON -DBUILD_WORKER=OFF -DPROTO_FILES_DIR=/app/libarmonik/Protos \ 
          -DCPACK_GENERATOR=TGZ /app/libarmonik && \
    make -j $(nproc) && \
    make package -j
