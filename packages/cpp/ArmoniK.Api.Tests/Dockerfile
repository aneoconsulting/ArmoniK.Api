FROM alpine:3.24 AS builder

RUN apk add --no-cache \
    build-base \
    cmake \
    linux-headers \
    c-ares-dev \
    grpc-dev \
    protobuf-dev \
    curl-dev

WORKDIR /app/source
COPY packages/cpp/tools/packaging/common/. ./tools/packaging/common/
COPY Protos/V1/. ./Protos/
COPY packages/cpp/ArmoniK.Api.Common/. ./ArmoniK.Api.Common/
COPY packages/cpp/ArmoniK.Api.Client/. ./ArmoniK.Api.Client/
COPY packages/cpp/ArmoniK.Api.Tests/. ./ArmoniK.Api.Tests/
COPY packages/cpp/CMakeLists.txt .
COPY packages/cpp/Packaging.cmake .
COPY packages/cpp/Dependencies.cmake .

WORKDIR /app/build
RUN cmake -DCMAKE_BUILD_TYPE=Debug -DPROTO_FILES_DIR=/app/source/Protos -DCMAKE_INSTALL_PREFIX=/app/install -DBUILD_WORKER=OFF -DBUILD_TEST=ON /app/source/ \
    && make -j "$(nproc)" install

# Minimal musl runtime: binaries, shared libraries they need, no shell, no apk
COPY packages/cpp/tools/make-rootfs.sh /usr/local/bin/make-rootfs.sh
RUN sh /usr/local/bin/make-rootfs.sh /rootfs /app/install \
    && mkdir -p /rootfs/app/install/lib \
    && cp -a /app/install/bin /rootfs/app/install/ \
    && find /app/install/lib -name '*.so*' -exec cp -a {} /rootfs/app/install/lib/ \;

FROM scratch
COPY --from=builder /rootfs/ /

ENV LD_LIBRARY_PATH="/app/install/lib"
ENV PATH="/app/install/bin"
# Trust store shared by gRPC and the tests' libcurl calls. There is no
# update-ca-certificates in this image: to add a CA, mount a full bundle over
# this file (see tools/run_test.sh).
ENV GRPC_DEFAULT_SSL_ROOTS_FILE_PATH="/etc/ssl/certs/ca-certificates.crt"
ENV SSL_CERT_FILE="/etc/ssl/certs/ca-certificates.crt"
USER 5000:5000

ENTRYPOINT ["/app/install/bin/ArmoniK.Api.Tests"]
