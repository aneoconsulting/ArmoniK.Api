# Connecting to ArmoniK Securely using gRPC in Python

## Overview

This guide provides steps to connect to ArmoniK securely using gRPC in Python. ArmoniK supports both TLS and mTLS (mutual TLS) for secure communication.

## Prerequisites

Before proceeding, make sure that ArmoniK is deployed with the necessary certificates. Follow the steps below:

1. Ensure that ArmoniK is deployed with authentication. Refer to the [How to configure authentication in ArmoniK](https://aneoconsulting.github.io/ArmoniK/guide/how-to/how-to-configure-authentication#how-to-configure-authentication) guide for detailed instructions.

2. Deploy ArmoniK.

3. Certificates will be generated:

### TLS

- `ca.crt`: Certificate Authority root certificate

### mTLS

- `ca.crt`: Certificate Authority root certificate
- `client.submitter.crt`: Client certificate for submission
- `client.submitter.key`: Private key corresponding to the client certificate

## Create Credentials for a Secure Channel

### Use the provided certificates to establish secure channel credentials

When interacting with ArmoniK using Python, use the Common name as the endpoint. Ensure that the certificate's Common Name (CN) matches the endpoint name.

### If the given certificate common name doesn't match the endpoint name (Armonik default certificates for example)

Update your system's hosts file to associate the ArmoniK control plane address with the domain name "armonik.local". Use the following command to edit the hosts file:

```bash
sudo nano /etc/hosts
```

Use `armonik.local` as endpoint and don't forget to specify the port

```bash
armonik.local:5001
```

## Launching the Python Script

Once you have configured ArmoniK and updated your hosts file, you can execute the example script from the root. Ensure that you have the Armonik Python dependencie installed.

```bash
pip install armonik
```

1. **For Insecure Channel**

    ```bash
    python examples/python/secure_grpc_client.py
    ```

2. **For TLS Secure Channel**

    ```bash
    python examples/python/secure_grpc_client.py --endpoint armonik.local:5001 --ssl [--ca <ca.crt path>]
    ```

3. **For Mutual TLS Secure Channel**

    ```bash
    python examples/python/secure_grpc_client.py --endpoint armonik.local:5001 --ssl [--ca <ca.crt path>] --key <client.submitter.key path> --cert <client.submitter.crt>
    ```
