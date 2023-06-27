#!/bin/bash

# Set the image tag for Docker
IMAGE_TAG="ubuntu-grpc:v0.1"

# Get the absolute path of the current script and its directory
script_path=$(readlink -f "${BASH_SOURCE:-$0}")
script_dir=$(dirname $script_path)
echo $script_dir

# Set the working directory to the parent directory of the script
working_dir=$script_dir/../
cd $working_dir
working_dir=$(pwd -P)
cd -

# Set the path to the protocol buffer (Protos) directory
proto_path=${script_dir}/../../../Protos/V1/

# Change to the Protos directory and store its absolute path
cd $proto_path
proto_dir=$(pwd -P)
cd -

# Create a build directory and store its absolute path
mkdir -p ${working_dir}/build
cd ${working_dir}/build
build_dir=$(pwd -P)
cd -

# Create an install directory and store its absolute path
mkdir -p ${working_dir}/install
cd ${working_dir}/install
install_dir=$(pwd -P)
cd -

# Display the directories
echo "Working dir          : ${working_dir}"
echo "Directory of proto   : ${proto_dir}"
echo "Directory of build   : ${build_dir}"
echo "Directory of install : ${install_dir}"

# Change to the working directory
cd ${working_dir}

# Check if the Docker image exists, and if not, build it
if [[ "$(docker images -q ${IMAGE_TAG} 2> /dev/null)" == "" ]]; then
  echo "Build docker image ${IMAGE_TAG} to compile Armonik.Api.Cpp"
  docker build -t ${IMAGE_TAG} -f tools/Dockerfile.ubuntu .
fi

# Compile the project source using the Docker image
echo "Compiling project source"
docker run -v ${proto_dir}:/app/proto -v ${working_dir}:/app/source -v ${install_dir}:/app/install --rm -it ${IMAGE_TAG}
