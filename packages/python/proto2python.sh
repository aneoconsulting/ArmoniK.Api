#!/usr/bin/env bash

# Bash script to create the python packages from the grpc proto for Armonik.Api
# We are using the same structure than the C# package

if [ "$1" = "-h" ] || [ "$1" = "--help" ] || [ -z "$1" ]      # Request help.
then
    echo "usage: $0 <path where the python virtual env will be created>"
    exit
else
    export PYTHON_VENV="$1" 
fi;

source ../common/protofiles.sh

export PATH=$HOME/.local/bin:$PATH
export ARMONIK_PYTHON_SRC="generated"
export PACKAGE_PATH="pkg"
export ARMONIK_WORKER=$ARMONIK_PYTHON_SRC"/armonik/worker"
export ARMONIK_CLIENT=$ARMONIK_PYTHON_SRC"/armonik/client"
export ARMONIK_COMMON=$ARMONIK_PYTHON_SRC"/armonik/common"

mkdir -p $ARMONIK_WORKER $ARMONIK_CLIENT $ARMONIK_COMMON $PACKAGE_PATH

# for debian/ubuntu if you don't have python 3 installed:
# sudo apt install python3-venv python3 python-is-python3 python3-pip

python -m pip install --upgrade pip
python -m venv $PYTHON_VENV
source $PYTHON_VENV/bin/activate
python -m pip install build grpcio grpcio-tools fix-protobuf-imports

unset proto_files
for proto in ${armonik_worker_files[@]}; do
    proto_files="$PROTO_PATH/$proto $proto_files"
done
python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
        --python_out=$ARMONIK_WORKER --grpc_python_out=$ARMONIK_WORKER \
        $proto_files

unset proto_files
for proto in ${armonik_client_files[@]}; do
    proto_files="$PROTO_PATH/$proto $proto_files" 
done
python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
        --python_out=$ARMONIK_CLIENT --grpc_python_out=$ARMONIK_CLIENT \
        $proto_files

unset proto_files
for proto in ${armonik_common_files[@]}; do
    proto_files="$PROTO_PATH/$proto $proto_files" 
done
python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
        --python_out=$ARMONIK_COMMON --grpc_python_out=$ARMONIK_COMMON \
        $proto_files

touch $ARMONIK_WORKER/__init__.py
touch $ARMONIK_CLIENT/__init__.py
touch $ARMONIK_COMMON/__init__.py

# Need to fix the relative import
# the package fix_protobuf_import help a lot but miss the capactiy to do the same things for the _pb2_grpc.py file
sed -i 's/\_pb2\.py/\_pb2\*\.py/g' $PYTHON_VENV//lib/python*/site-packages/fix_protobuf_imports/*.py
fix-protobuf-imports $ARMONIK_PYTHON_SRC/armonik

# another fix to have working relative import
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_WORKER/*
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_CLIENT/*
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_COMMON/*

# Build cannot access files outside the current directory
cp $README_PATH $ARMONIK_PYTHON_SRC

python -m build -s -w -o $PACKAGE_PATH