#! /bin/sh

set -x
script_path="$(dirname "${BASH_SOURCE:-$0}")"
CertFolder="$(realpath "$script_path/../../csharp/certs" )"
docker run --rm -t --network host -v "$CertFolder:/app/source/certs" -v "/usr/local/share/ca-certificates/:/usr/local/share/ca-certificates" \
            -e GrpcClient__Endpoint \
            -e Http__Endpoint="${Http__Endpoint:-$Grpc__Endpoint}" \
            ${GrpcClient__AllowUnsafeConnection:+-e GrpcClient__AllowUnsafeConnection} \
            ${GrpcClient__CaCert:+-e GrpcClient__CaCert="/app/source/certs/server1-ca.pem"} \
            ${GrpcClient__CertPem:+-e GrpcClient__CertPem="/app/source/certs/client.pem"} \
            ${GrpcClient__KeyPem:+-e GrpcClient__KeyPem="/app/source/certs/client.key"} \
            "armonik-api-cpp:0.1.0"
