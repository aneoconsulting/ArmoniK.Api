#!/bin/bash

# Declare a variable that will store the image tag for our Docker image
IMAGE_TAG="armonik-api-cpp:v0.1"

# Determine the full path of the script being executed
script_path=$(readlink -f "${BASH_SOURCE:-$0}")

# Extract the directory from the script path
script_dir=$(dirname $script_path)

# Print the script directory
echo $script_dir

# Set the working directory to be three levels above the script directory
working_dir=$script_dir/../../../

# Change to the working directory
cd $working_dir

# Get the full path of the current working directory
working_dir=$(pwd -P)

# Print a message explaining that we are building the worker image in the root directory
echo "To build worker image. Change to root directory where Protos are"

# Print the working directory
echo "Change to directory [${working_dir}]"

# Build the Docker image using the specified Dockerfile and tag it with the IMAGE_TAG variable
docker build --rm -t ${IMAGE_TAG} -f packages/cpp/tools/Dockerfile.worker .
