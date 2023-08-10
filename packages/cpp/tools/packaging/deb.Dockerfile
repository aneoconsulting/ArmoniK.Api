FROM debian:12

# Install debian package making tools
RUN apt-get update && DEBIAN_FRONTEND="noninteractive" TZ="Europe/London" apt-get install --no-install-recommends -y \
    debhelper \
    debmake \
    devscripts \
    equivs  \
    && apt-get clean

WORKDIR /app/libarmonik
COPY --chmod=755 packages/cpp/tools/packaging/debian/. ./debian/
# Install build dependencies
RUN yes | mk-build-deps -i -r -B
COPY packages/cpp/tools/packaging/BundledLICENSE ./LICENSE
COPY Protos/V1/. ./Protos/
COPY packages/cpp/ArmoniK.Api.Common/. ./ArmoniK.Api.Common/
COPY packages/cpp/ArmoniK.Api.Client/. ./ArmoniK.Api.Client/
COPY packages/cpp/ArmoniK.Api.Worker/. ./ArmoniK.Api.Worker/
COPY packages/cpp/CMakeLists.txt .

ENV DEBEMAIL="armonik-support@aneo.fr"
ENV DEBFULLNAME="ANEO Consulting"
ARG VERSION="3.11.0"
# Prepare package
RUN debmake -t -p "libarmonik" -u "$VERSION" -b libarmonik:lib
WORKDIR /app/"libarmonik-$VERSION"
# Build package
RUN debuild -i -us -uc -b -D
