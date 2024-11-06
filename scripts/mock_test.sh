#! /bin/sh

set -x

if [ $# -lt 2 ]; then
  echo "Usage: $0 <test_dir> <test_command>"
  exit 1
fi

script_path="$(dirname "${BASH_SOURCE:-$0}")"
working_dir="$(realpath "$script_path/../packages" )"

TEST_DIR="${1:?Test dir is not set}"
TEST_COMMAND="${2:?Command is not set}"

if [ -n "$Grpc__CaCert" ]; then
  export Grpc__CaCert="$working_dir/csharp/$Grpc__CaCert"
fi
if [ -n "$Grpc__ClientCert" ]; then
  export Grpc__ClientCert="$working_dir/csharp/$Grpc__ClientCert"
fi
if [ -n "$Grpc__ClientKey" ]; then
  export Grpc__ClientKey="$working_dir/csharp/$Grpc__ClientKey"
fi

"$working_dir/csharp/out/ArmoniK.Api.Mock.exe" &
  server_pid=$!
sleep 5

cd "$working_dir/$TEST_DIR"

$TEST_COMMAND || ret=$?

echo $server_pid
kill $server_pid
exit $ret
