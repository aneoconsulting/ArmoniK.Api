#!/bin/bash

if [ $# -lt 2 ]; then
  echo "Usage: $0 <test_dir> <test_command>"
  exit 1
fi

script_path="$(dirname "${BASH_SOURCE:-$0}")"
working_dir="$(realpath "$script_path/../packages" )"

TEST_DIR=$1
TEST_COMMAND=$2

if [ -n "$Grpc__CaCert" ]; then
  export Grpc__CaCert=$working_dir/csharp/$Grpc__CaCert
fi
if [ -n "$Grpc__ClientCert" ]; then
  export Grpc__ClientCert=$working_dir/csharp/$Grpc__ClientCert
fi
if [ -n "$Grpc__ClientKey" ]; then
  export Grpc__ClientKey=$working_dir/csharp/$Grpc__ClientKey
fi

cd $working_dir/csharp
set +e
set -x
./out/ArmoniK.Api.Mock.exe &
  server_pid=$!
  sleep 5

set -e

cd $working_dir
cd $TEST_DIR

$TEST_COMMAND
ret=$?
set +e

echo $server_pid
kill $server_pid
exit $ret
