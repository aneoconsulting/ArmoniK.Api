FROM dockerhubaneo/armonikworker-base:ubi7.9-0.0.1

USER root
# Update the PATH environment variable to include the gRPC libraries and binaries
ENV LD_LIBRARY_PATH="/app/install/lib:$LD_LIBRARY_PATH"
ENV PATH="/app/install/bin:$PATH"

RUN yum --disableplugin=subscription-manager check-update \
    ; yum --disableplugin=subscription-manager \
        install -y git make \
        rh-python38-python-devel \
        centos-release-scl \
        devtoolset-10 \
        rpmdevtools \
        rpmlint \
        && yum --disableplugin=subscription-manager clean all
RUN ln -s /opt/cmake-3.24.1/bin/* /usr/local/bin
RUN echo "source /opt/rh/devtoolset-10/enable" >> /etc/bashrc
SHELL ["/bin/bash", "--login", "-c"]

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
RUN cmake -DBUILD_SHARED_LIBS=ON -DBUILD_CLIENT:BOOL=ON -DCMAKE_BUILD_TYPE=Release -DBUILD_WORKER:BOOL=ON -DPROTO_FILES_DIR=/rpm/Protos -DCPACK_GENERATOR=RPM -DCMAKE_PREFIX_PATH=/usr/local/grpc .. && make package -j
ENTRYPOINT ["bash"]

