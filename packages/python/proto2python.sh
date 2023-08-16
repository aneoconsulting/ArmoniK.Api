#!/usr/bin/env bash

set -e

# Bash script to create the python packages from the grpc proto for ArmoniK.Api
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
export ARMONIK_PYTHON_SRC="src"
export ARMONIK_MODULE_PATH=$ARMONIK_PYTHON_SRC"/armonik"
export PACKAGE_PATH="pkg"
export GENERATED_PATH=$ARMONIK_MODULE_PATH"/protogen"
export ARMONIK_WORKER=$GENERATED_PATH"/worker"
export ARMONIK_CLIENT=$GENERATED_PATH"/client"
export ARMONIK_COMMON=$GENERATED_PATH"/common"

mkdir -p $ARMONIK_WORKER $ARMONIK_CLIENT $ARMONIK_COMMON $PACKAGE_PATH

# for debian/ubuntu if you don't have python 3 installed:
# sudo apt install python3-venv python3 python-is-python3 python3-pip

python -m pip install --upgrade pip
python -m venv $PYTHON_VENV
source $PYTHON_VENV/bin/activate
python -m pip install build grpcio grpcio-tools click pytest setuptools_scm[toml]

unset proto_files
for proto in ${armonik_worker_files[@]}; do
    proto_files="$PROTO_PATH/$proto $proto_files"
done
python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
        --python_out=$ARMONIK_WORKER --grpc_python_out=$ARMONIK_WORKER --pyi_out=$ARMONIK_WORKER \
        $proto_files

unset proto_files
for proto in ${armonik_client_files[@]}; do
    proto_files="$PROTO_PATH/$proto $proto_files"
done
python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
        --python_out=$ARMONIK_CLIENT --grpc_python_out=$ARMONIK_CLIENT --pyi_out=$ARMONIK_CLIENT \
        $proto_files

unset proto_files
for proto in ${armonik_common_files[@]}; do
    proto_files="$PROTO_PATH/$proto $proto_files"
done
python -m grpc_tools.protoc -I $PROTO_PATH --proto_path=$PROTO_PATH \
        --python_out=$ARMONIK_COMMON --grpc_python_out=$ARMONIK_COMMON --pyi_out=$ARMONIK_COMMON \
        $proto_files

touch $ARMONIK_WORKER/__init__.py
touch $ARMONIK_CLIENT/__init__.py
touch $ARMONIK_COMMON/__init__.py

# Need to fix the relative import
python fix_imports.py $GENERATED_PATH

export GENVERSION_OPT="-w $ARMONIK_MODULE_PATH/_version.py"

if [ "$CI" != "" ]
then
	export GENVERSION_OPT="$GENVERSION_OPT -n"
fi

if [ "$RELEASE" == "" ] && [ "$GITHUB_RUN_ID" != "" ]
then
	export GENVERSION_OPT="$GENVERSION_OPT -d $GITHUB_RUN_NUMBER"
fi

python genversion.py $GENVERSION_OPT

python -m build -w -s -o $PACKAGE_PATH
