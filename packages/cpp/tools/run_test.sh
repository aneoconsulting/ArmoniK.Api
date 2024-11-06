#! /bin/sh

set -x
script_path="$(dirname "${BASH_SOURCE:-$0}")"
CertFolder="$(realpath "$script_path/../../csharp/certs" )"
docker run --rm -t --network host -v "$CertFolder:/app/source/certs" -v "/usr/local/share/ca-certificates/:/usr/local/share/ca-certificates" \
            -e Grpc__EndPoint="$Grpc__Endpoint" \
            -e Http__EndPoint="${Http__Endpoint:-$Grpc__Endpoint}" \
            ${Grpc__SSLValidation:+-e Grpc__SSLValidation} \
            ${Grpc__CaCert:+-e Grpc__CaCert="/app/source/certs/server1-ca.pem"} \
            ${Grpc__mTLS:+-e Grpc__mTLS} \
            ${Grpc__ClientCert:+-e Grpc__ClientCert="/app/source/certs/client.pem"} \
            ${Grpc__ClientKey:+-e Grpc__ClientKey="/app/source/certs/client.key"} \
            "armonik-api-cpp:0.1.0"
