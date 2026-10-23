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
# Checked before the server starts so a missing endpoint or curl does not leave it running.
HTTP_ENDPOINT="${Http__Endpoint:?Http__Endpoint is not set}"
command -v curl >/dev/null || { echo "curl is required to wait for the mock server"; exit 1; }

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
# An endpoint is ready once curl opens a TCP connection to it (`num_connects`), whatever happens next:
# a TLS handshake refused for lack of a client certificate under mTLS, or an HTTP/1.1 request on the
# HTTP/2-only gRPC port, still proves it listens. A refused connection, a bad URL or an unresolved host
# opens none and keeps waiting until the timeout.
# gRPC listens on its own port in the noTLS step, so its endpoint is probed as well.
endpoints="$HTTP_ENDPOINT"
if [ -n "$Grpc__Endpoint" ] && [ "$Grpc__Endpoint" != "$HTTP_ENDPOINT" ]; then
  endpoints="$endpoints $Grpc__Endpoint"
fi
startup_timeout="${MOCK_STARTUP_TIMEOUT:-60}"
waited=0
while :; do
  ready=true
  for endpoint in $endpoints; do
    connects="$(curl --silent --insecure --output /dev/null --max-time 2 --write-out '%{num_connects}' "$endpoint")"
    [ "${connects:-0}" -gt 0 ] || ready=false
  done
  # Checked after the probes too: a server that failed to bind exits, and the probes alone cannot tell
  # it from another process still holding the port.
  if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "Mock server exited before accepting connections"
    exit 1
  fi
  if [ "$ready" = true ]; then
    break
  fi
  if [ "$waited" -ge "$startup_timeout" ]; then
    echo "Mock server not ready on $endpoints after ${startup_timeout}s"
    kill "$server_pid"
    wait "$server_pid"
    exit 1
  fi
  sleep 1
  waited=$((waited + 1))
done

cd "$working_dir/$TEST_DIR"

ret=0
$TEST_COMMAND || ret=$?

echo $server_pid
kill $server_pid
# The next step reuses the port: let it find the port free, not this server still shutting down.
wait $server_pid
exit $ret
