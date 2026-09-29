# The build environment of this slice on the NixOS campaign machine (2026-09-29): the
# system's nixpkgs (NIX_PATH), so the versions are whatever that channel pins. Every header
# records the versions actually used (protobuf, grpc++, g++, cmake, rustc). cargo and rustc
# are NOT from here: the ambient toolchain on PATH builds the core, as it builds the shared
# server (poc/rust/serve.sh build), so both see one rustc.
#
#   nix-shell gen/shell.nix --run 'cmake ...'
#
# grpc here is the channel's current gRPC (1.80.0 with protobuf 34.1 on 2026-09-29): the
# "current version" of CAMPAIGN.md section 3, not ArmoniK's v1.54.0 (that one needs its own
# prefix, AK_INCUMBENT_PREFIX).
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  packages = with pkgs; [
    cmake
    pkg-config
    grpc
    protobuf
    abseil-cpp
    openssl
    zlib
    c-ares
    re2
    git
    python3
  ];
}
