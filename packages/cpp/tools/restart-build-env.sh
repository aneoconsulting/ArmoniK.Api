#!/usr/bin/env bash

set -x

IMAGE_NAME="armonik_api_build_env"
IMAGE_TAG="0.1.0"
CONTAINER_NAME="armonik_api_env"

docker stop "/${CONTAINER_NAME}"

script_path="$(readlink -f "${BASH_SOURCE:-$0}")"
script_dir="$(dirname "$script_path")"

working_dir="$script_dir/../"
cd "$working_dir"
working_dir="$(pwd -P)"
cd -

proto_path="${script_dir}/../../../Protos/V1/"

# Change to the Protos directory and store its absolute path
cd $proto_path
proto_dir="$(pwd -P)"
cd -

# Create an install directory and store its absolute path
mkdir -p "${working_dir}/install"
cd "${working_dir}/install"
install_dir="$(pwd -P)"
cd -

docker build -t "${IMAGE_NAME}:${IMAGE_TAG}" -f BuildEnv.Dockerfile .

# Change to the working directory
cd "${working_dir}"

mkdir -p ${working_dir}/build
mkdir -p ${install_dir}

REMOTE_BUILD_ADDRESS="${REMOTE_BUILD_ADDRESS:-"127.0.0.1:2223"}"
docker run --rm -d --cap-add sys_ptrace -p"${REMOTE_BUILD_ADDRESS}":22 --name "${CONTAINER_NAME}" -v "${proto_dir}:/app/proto" -v "${working_dir}:/app/source" -v "${install_dir}:/app/install" -v "${working_dir}/build:/app/build" "${IMAGE_NAME}:${IMAGE_TAG}"
