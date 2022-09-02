#!/usr/bin/env bash

# Bash script to create the python packages from the grpc proto for Armonik.Api
# We are using the same structure than the C# package

#export PATH=$HOME/.local/bin:$PATH

export PROTO_PATH="Protos/V1"
export ARMONIK_PYTHON_SRC="Api/python/src" 
export ARMONIK_SERVER=$ARMONIK_PYTHON_SRC"/armonik/server"
export ARMONIK_CLIENT=$ARMONIK_PYTHON_SRC"/armonik/client"
export ARMONIK_MESSAGE=$ARMONIK_PYTHON_SRC"/armonik/message"

mkdir -p $ARMONIK_SERVER $ARMONIK_CLIENT $ARMONIK_MESSAGE

# for debian/ubuntu:
# sudo apt install python3-venv

armonik_server_files=("worker_service.proto")
armonik_client_files=("agent_service.proto" "submitter_service.proto")
armonik_message_files=("objects.proto" "task_status.proto" "session_status.proto" "result_status.proto")

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

for proto in ${armonik_message_files[@]}; do
    python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
            --python_out=$ARMONIK_MESSAGE --grpc_python_out=$ARMONIK_MESSAGE \
            $PROTO_PATH/$proto
done

touch $ARMONIK_SERVER/__input__.py
touch $ARMONIK_CLIENT/__input__.py
touch $ARMONIK_MESSAGE/__input__.py

#cd $ARMONIK_PYTHON_SRC
fix-protobuf-imports $ARMONIK_PYTHON_SRC/armonik

# another fix to have working relative import
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_SERVER/*
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_CLIENT/*
sed -i 's/from \.\.\./from \.\./g' $ARMONIK_MESSAGE/*

#cd ../../..
python -m build