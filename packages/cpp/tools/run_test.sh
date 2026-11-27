#! /bin/sh

set -x
script_path="$(dirname "${BASH_SOURCE:-$0}")"
CertFolder="$(realpath "$script_path/../../csharp/certs" )"

# On Windows the tests are built natively (see test.yml), run them directly
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    exec "$script_path/../build/ArmoniK.Api.Tests/Release/ArmoniK.Api.Tests.exe"
    ;;
  *)
    # The image has no shell nor update-ca-certificates: mount the host trust store
    # (which CI extends with update-ca-certificates) and run as the caller so the
    # mounted keys stay readable.
    docker run --rm -t --network host --user "$(id -u):$(id -g)" -v "$CertFolder:/app/source/certs:ro" \
                -v "/etc/ssl/certs/ca-certificates.crt:/etc/ssl/certs/ca-certificates.crt:ro" \
                -e GrpcClient__Endpoint \
                -e Http__Endpoint="${Http__Endpoint:-$Grpc__Endpoint}" \
                ${GrpcClient__AllowUnsafeConnection:+-e GrpcClient__AllowUnsafeConnection} \
                ${GrpcClient__CaCert:+-e GrpcClient__CaCert="/app/source/certs/server1-ca.pem"} \
                ${GrpcClient__CertPem:+-e GrpcClient__CertPem="/app/source/certs/client.pem"} \
                ${GrpcClient__KeyPem:+-e GrpcClient__KeyPem="/app/source/certs/client.key"} \
                "armonik-api-cpp:0.1.0"
    ;;
esac
