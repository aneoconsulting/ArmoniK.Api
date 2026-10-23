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
# Checked before the server starts so a missing endpoint does not leave it running.
HTTP_ENDPOINT="${Http__Endpoint:?Http__Endpoint is not set}"

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

# Wait until the mock server accepts connections rather than for a fixed delay: a slow start, common
# on Windows runners, made the first tests fail with "connection refused".
# Only curl's "couldn't connect" (7) and "timeout" (28) mean it is not listening yet. Any other
# outcome, including a TLS handshake refused for lack of a client certificate under mTLS, means it is.
startup_timeout="${MOCK_STARTUP_TIMEOUT:-60}"
waited=0
while :; do
  curl --silent --insecure --output /dev/null --max-time 2 "$HTTP_ENDPOINT"
  case $? in
    7|28) ;;
    *) break ;;
  esac
  if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "Mock server exited before accepting connections"
    exit 1
  fi
  if [ "$waited" -ge "$startup_timeout" ]; then
    echo "Mock server not ready after ${startup_timeout}s"
    kill "$server_pid"
    exit 1
  fi
  sleep 1
  waited=$((waited + 1))
done

cd "$working_dir/$TEST_DIR"

$TEST_COMMAND || ret=$?

echo $server_pid
kill $server_pid
exit $ret
