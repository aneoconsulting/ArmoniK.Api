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

# Wait for the mock to answer rather than assuming it started: a mock that failed to come up used
# to look like a test failure in whichever language was running.
endpoint="${GrpcClient__Endpoint:-http://localhost:5000}"
waited=0
until curl --silent --output /dev/null --insecure --max-time 1 "$endpoint/calls.json"; do
  if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "the mock server exited before it began answering" >&2
    exit 1
  fi
  waited=$((waited + 1))
  if [ "$waited" -ge 15 ]; then
    echo "the mock server is not answering at $endpoint after ${waited}s" >&2
    kill "$server_pid"
    exit 1
  fi
  sleep 1
done

cd "$working_dir/$TEST_DIR"

$TEST_COMMAND || ret=$?

echo $server_pid
kill $server_pid
exit $ret
