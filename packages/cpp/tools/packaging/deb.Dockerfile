FROM ubuntu:23.04

# Install dependencies
RUN apt-get update && DEBIAN_FRONTEND="noninteractive" TZ="Europe/London" apt-get install -y \
    gcc \
    g++ \
    make \
    build-essential \
    cmake \
    libc-ares-dev \
    protobuf-compiler-grpc \
    grpc-proto \
    libgrpc-dev \
    libgrpc++-dev \
    debhelper \
    debmake \
    quilt

WORKDIR /app/libarmonik
COPY Protos/V1/. ./Protos/
COPY packages/cpp/ArmoniK.Api.Common/. ./ArmoniK.Api.Common/
COPY packages/cpp/ArmoniK.Api.Client/. ./ArmoniK.Api.Client/
COPY packages/cpp/ArmoniK.Api.Worker/. ./ArmoniK.Api.Worker/
COPY packages/cpp/CMakeLists.txt .
COPY packages/cpp/NOTICE.txt .
COPY --chmod=755 packages/cpp/tools/packaging/debian/. ./debian/

ARG PACKAGE_NAME="libarmonik"
ARG VERSION="0.1.0"

RUN debmake -T -t -f "ANEO Consulting" -e "armonik-support@aneo.fr" -p "$PACKAGE_NAME" -u "$VERSION" -b libarmonik:lib -i debuild

ENTRYPOINT [ "bash" ]

