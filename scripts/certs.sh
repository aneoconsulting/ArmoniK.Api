#! /bin/sh

set -ex

chain() {
  cat > "$1-ca.cnf" <<EOF
[ req ]
distinguished_name = req_distinguished_name
prompt = no
[ req_distinguished_name ]
countryName            = FR
stateOrProvinceName    = France
organizationName       = ArmoniK Ingress Root (NonTrusted)
organizationalUnitName = $1
commonName             = ArmoniK Ingress Root (NonTrusted) Private Certificate Authority
EOF
  openssl req -config "$1-ca.cnf" -x509 -newkey rsa:4096 -days 3650 -nodes -keyout "$1-ca.key" -out "$1-ca.pem"


  cat > "$1.cnf" <<EOF
[ req ]
distinguished_name = req_distinguished_name
${2:+req_extensions     = v3_req}
prompt = no
[ req_distinguished_name ]
countryName            = FR
stateOrProvinceName    = France
organizationName       = ArmoniK Ingress Root (NonTrusted)
organizationalUnitName = $1
commonName             = ${2:-ArmoniK Ingress Root (NonTrusted) User Certificate}
${2:+[ v3_req ]}
${2:+subjectAltName = @alt_names}
${2:+[ alt_names ]}
${2:+DNS.1 = $2}
EOF

  openssl genrsa -out "$1.key" 4096
  openssl req -config "$1.cnf" -new -key "$1".key -out "$1".csr ${2:+-extensions v3_req}

  openssl x509 -req -in "$1.csr" -CA "$1-ca.pem" -CAkey "$1-ca.key" -CAcreateserial -out "$1.pem" -days 3650 -extfile "$1.cnf" ${2:+-extensions v3_req}
}

chain server1 localhost
chain server2 localhost
chain client
