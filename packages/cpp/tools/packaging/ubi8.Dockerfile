FROM registry.access.redhat.com/ubi8

ARG PARALLEL_JOBS=4

RUN rpm -ivh https://dl.fedoraproject.org/pub/epel/epel-release-latest-8.noarch.rpm

RUN yum --disableplugin=subscription-manager update -y && \
    yum --disableplugin=subscription-manager install -y \
    make \
    cmake \
    git \
    gcc \
    gcc-c++ \
    wget \
    rpm-build \
    libcurl-devel \
    fmt-devel \
    simdjson-devel \
    re2-devel \
    zlib-devel \
    openssl-devel

RUN yum --disableplugin=subscription-manager clean all

WORKDIR /tmp

# Fetch aneo's grpc rpm packages
RUN wget https://github.com/aneoconsulting/grpc-rpm/releases/download/1.62.2.0/grpc-1.62.2-1.el8.x86_64.rpm && \
    rpm -ivh grpc-1.62.2-1.el8.x86_64.rpm && \
    wget https://github.com/aneoconsulting/grpc-rpm/releases/download/1.62.2.0/grpc-devel-1.62.2-1.el8.x86_64.rpm && \
    rpm -ivh grpc-devel-1.62.2-1.el8.x86_64.rpm && \
    rm -rf ./*.rpm

WORKDIR /rpm
RUN mkdir -p build

COPY packages/cpp/tools/packaging/common/. ./tools/packaging/common/.
COPY Protos/V1/. ./Protos/
COPY packages/cpp/ArmoniK.Api.Common/. ./ArmoniK.Api.Common/.
COPY packages/cpp/ArmoniK.Api.Client/. ./ArmoniK.Api.Client/.
COPY packages/cpp/ArmoniK.Api.Worker/. ./ArmoniK.Api.Worker/.
COPY packages/cpp/CMakeLists.txt .
COPY packages/cpp/Packaging.cmake .
COPY packages/cpp/Dependencies.cmake .

WORKDIR /rpm/build
RUN cmake \
    "-DBUILD_SHARED_LIBS=OFF" \
    "-DBUILD_CLIENT:BOOL=ON" \
    "-DCMAKE_BUILD_TYPE=Release" \
    "-DBUILD_WORKER:BOOL=ON" \
    "-DPROTO_FILES_DIR=/rpm/Protos" \
    "-DCPACK_GENERATOR=RPM" \
    "-DCMAKE_PREFIX_PATH=/usr/local/grpc" .. && make package -j $(PARALLEL_JOBS)
ENTRYPOINT ["bash"]

