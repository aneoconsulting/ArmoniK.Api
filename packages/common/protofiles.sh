#!/usr/bin/env bash

export REPOSITORY_PATH=$(pwd)/../..
export PROTO_PATH=$REPOSITORY_PATH/Protos/V1
export README_PATH=$REPOSITORY_PATH/README.md

armonik_worker_files=("agent_service.proto" "worker_service.proto")
if IFS=$'\n' read -rd '' -a armonik_client_files <<<"$(find $PROTO_PATH -name "*.proto" -exec basename {} \; | grep -v -e "worker" -e "agent" | grep "service" )"; then :; fi
if IFS=$'\n' read -rd '' -a armonik_common_files <<<"$(find $PROTO_PATH -name "*.proto" -exec basename {} \; | grep -v "service" )"; then :; fi
