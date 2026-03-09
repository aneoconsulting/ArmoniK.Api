#!/bin/sh

set -x

# Set the image tag for Docker
IMAGE_TAG="${1:-ubuntu-grpc:v0.1}"

# Get the absolute path of the current script and its directory
script_path="$(readlink -f "${BASH_SOURCE:-$0}")"
script_dir="$(dirname "$script_path")"

# Set the working directory to the parent directory of the script
working_dir="$script_dir/../"
cd "$working_dir"
working_dir="$(pwd -P)"
cd -

# Set the path to the protocol buffer (Protos) directory
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

# Change to the working directory
cd "${working_dir}"

# Check if the Docker image exists, and if not, build it
docker build -t "${IMAGE_TAG}" -f ${script_dir}/Dockerfile.ubuntu .

mkdir -p ${working_dir}/build
mkdir -p ${working_dir}/buildtest
mkdir -p ${install_dir}

# Compile the project source using the Docker image
docker run -v "${proto_dir}:/app/proto" -v "${working_dir}:/app/source" -v "${install_dir}:/app/install" -v "${working_dir}/build:/app/build" -v "${working_dir}/buildtest:/app/buildtest" --rm "${IMAGE_TAG}"
