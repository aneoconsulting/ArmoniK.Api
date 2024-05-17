#! /bin/sh

set -ex

chain() {
  openssl req -x509 -newkey rsa:4096 -days 3650 -nodes -keyout "$1-ca.key" -out "$1-ca.pem" -subj "/C=FR/ST=France/L=/O=ArmoniK Ingress Root (NonTrusted)/OU=/CN=ArmoniK Ingress Root (NonTrusted) Private Certificate Authority/emailAddress="

  openssl genrsa -out "$1.key" 4096
  openssl req -new -key "$1".key -out "$1".csr -subj "/C=FR/ST=France/L=/O=ArmoniK Ingress Root (NonTrusted)/OU=/CN=${2:-ArmoniK Root (NonTrusted}/emailAddress="

  cat > "$1.cnf" <<EOF
${2:+subjectAltName=DNS:$2}
EOF

  openssl x509 -req -in "$1.csr" -CA "$1-ca.pem" -CAkey "$1-ca.key" -CAcreateserial -out "$1.pem" -days 3650 -extfile "$1.cnf"
}

chain server localhost
chain client
