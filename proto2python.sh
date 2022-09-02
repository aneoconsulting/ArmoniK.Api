#!/usr/bin/env bash

# Bash script to create the python packages from the grpc proto for Armonik.Api
# We are using the same structure than the C# package

export PATH=$HOME/.local/bin:$PATH

export PROTO_PATH="Protos/V1"
export ARMONIK_PYTHON_SRC="Api/python/src" 
export ARMONIK_SERVER=$ARMONIK_PYTHON_SRC"/armonik/server"
export ARMONIK_CLIENT=$ARMONIK_PYTHON_SRC"/armonik/client"
export ARMONIK_COMMON=$ARMONIK_PYTHON_SRC"/armonik/common"

mkdir -p $ARMONIK_SERVER $ARMONIK_CLIENT $ARMONIK_COMMON

# for debian/ubuntu:
# sudo apt install python3-venv

armonik_server_files=("agent_service.proto" "worker_service.proto")
armonik_client_files=("submitter_service.proto" "tasks_service.proto" "sessions_service.proto")
armonik_common_files=("objects.proto" "task_status.proto" "session_status.proto" \
                      "result_status.proto" "agent_common.proto" "sessions_common.proto"  \
                      "submitter_common.proto"  "tasks_common.proto"  "worker_common.proto")

python -m pip install --upgrade pip
python -m venv $HOME/grpc2
python -m pip install build
python -m pip install grpcio grpcio-tools fix-protobuf-imports

for proto in ${armonik_server_files[@]}; do
    python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
            --python_out=$ARMONIK_SERVER --grpc_python_out=$ARMONIK_SERVER \
            $PROTO_PATH/$proto
done

for proto in ${armonik_client_files[@]}; do
    python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
            --python_out=$ARMONIK_CLIENT --grpc_python_out=$ARMONIK_CLIENT \
            $PROTO_PATH/$proto
done

for proto in ${armonik_common_files[@]}; do
    python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
            --python_out=$ARMONIK_COMMON --grpc_python_out=$ARMONIK_COMMON \
            $PROTO_PATH/$proto
done

for proto in ${armonik_server_files[@]}; do
    python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
            --python_out=$ARMONIK_SERVER --grpc_python_out=$ARMONIK_SERVER \
            $PROTO_PATH/$proto
done

touch $ARMONIK_SERVER/__input__.py
touch $ARMONIK_CLIENT/__input__.py
touch $ARMONIK_COMMON/__input__.py

# Need to fix the relative import
# the package fix_protobuf_import help a lot but miss the capactiy to do the same things for the _pb2_grpc.py file
sed 's/\_pb2\.py/\_pb2\*\.py/g' $HOME/.local/lib/python*/site-packages/fix_protobuf_imports/*.py
fix-protobuf-imports $ARMONIK_PYTHON_SRC/armonik

# another fix to have working relative import
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_SERVER/*
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_CLIENT/*
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_COMMON/*

python -m build