FROM ubuntu:lunar-20231128

RUN apt-get update && DEBIAN_FRONTEND="noninteractive" TZ="Europe/London" apt-get install -y \
    ssh \
    gcc \
    g++ \
    gdb \
    clang \
    make \
    ninja-build \
    cmake \
    autoconf \
    automake \
    locales-all \
    build-essential \
    libc-ares-dev \
    protobuf-compiler-grpc \
    grpc-proto \
    libgrpc-dev \
    libgrpc++-dev \
    libprotobuf-dev \
    libfmt-dev \
    libcurl4-openssl-dev \
	&& apt-get clean

ENV protobuf_BUILD_TESTS=OFF

RUN ( \
    echo 'LogLevel DEBUG2'; \
    echo 'PermitRootLogin yes'; \
    echo 'PasswordAuthentication yes'; \
    echo 'Subsystem sftp /usr/lib/openssh/sftp-server'; \
  ) > /etc/ssh/sshd_config_test_clion \
  && mkdir -p /run/sshd

RUN yes password | passwd root

ENTRYPOINT ["/usr/sbin/sshd", "-D", "-e", "-f", "/etc/ssh/sshd_config_test_clion"]
