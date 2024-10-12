#!/bin/bash

if [ $# -lt 3 ]; then
  echo "Usage: $0 <mock_env> <test_env> <test_command>"
  exit 1
fi

script_path="$(dirname "${BASH_SOURCE:-$0}")"
script_dir="$(realpath "$script_path/" )"
working_dir="$(realpath "$script_path/../" )"

MOCK_ENV=$1
TEST_ENV=$2
TEST_COMMAND=$3

if [ ! -f "$MOCK_ENV" ]; then
  echo "Fichier d'environnement $MOCK_ENV introuvable"
  exit 1
fi

if [ ! -f "$TEST_ENV" ]; then
  echo "Fichier d'environnement $TEST_ENV introuvable"
  exit 1
fi

set -a
source "$MOCK_ENV"
set +a

cd $working_dir
set +e
set -x
./out/ArmoniK.Api.Mock.exe &
  server_pid=$!
  sleep 5

set -e

cd $script_dir
set -a
source "$TEST_ENV"
set +a

$TEST_COMMAND
set +e

echo $server_pid
kill $server_pid
