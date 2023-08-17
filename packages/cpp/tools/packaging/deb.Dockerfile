FROM debian:12

# Install debian package making tools
RUN apt-get update && DEBIAN_FRONTEND="noninteractive" TZ="Europe/London" apt-get install --no-install-recommends -y \
    devscripts \
    equivs \
    && apt-get clean

WORKDIR /app/libarmonik
COPY packages/cpp/tools/packaging/debian/control ./tools/packaging/debian/control
# Install build dependencies
RUN yes | mk-build-deps -i -r -B ./tools/packaging/debian/control
COPY packages/cpp/tools/packaging/common/. ./tools/packaging/common/
COPY Protos/V1/. ./Protos/
COPY packages/cpp/ArmoniK.Api.Common/. ./ArmoniK.Api.Common/
COPY packages/cpp/ArmoniK.Api.Client/. ./ArmoniK.Api.Client/
COPY packages/cpp/ArmoniK.Api.Worker/. ./ArmoniK.Api.Worker/
COPY packages/cpp/CMakeLists.txt .
COPY packages/cpp/Packaging.cmake .

WORKDIR /app/libarmonik/build
RUN cmake -DBUILD_SHARED_LIBS=ON -DBUILD_CLIENT:BOOL=ON -DCMAKE_BUILD_TYPE=Release -DBUILD_WORKER:BOOL=ON -DPROTO_FILES_DIR=/app/libarmonik/Protos -DCPACK_GENERATOR=DEB .. && make package -j && make clean
ENTRYPOINT ["bash"]
