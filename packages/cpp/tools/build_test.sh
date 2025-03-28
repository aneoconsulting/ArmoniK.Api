#! /bin/sh

script_path="$(dirname "${BASH_SOURCE:-$0}")"
working_dir="$(realpath "${script_path}/../../../" )"
dockerfile="${1:-"${working_dir}/packages/cpp/ArmoniK.Api.Tests/Dockerfile"}"
image_tag="${2:-"armonik-api-cpp:0.1.0"}"
docker build --rm -t "$image_tag" -f "$dockerfile" --progress plain "$working_dir"
