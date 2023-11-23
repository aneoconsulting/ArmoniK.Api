# Connecting to ArmoniK Securely using gRPC in Python

## Overview

This guide provides steps to connect to ArmoniK securely using gRPC in Python. ArmoniK supports both TLS and mTLS (mutual TLS) for secure communication.

### Prerequisites

Before proceeding, make sure that ArmoniK is deployed with the necessary certificates. Follow the steps below:

1. In the `parameters.tfvars` file of ArmoniK, set the following values in the Deploy Ingress section:

    For TLS only:

    ```hcl
    tls = true
    mtls = false
    ```

    For mTLS:

    ```hcl
    tls = true
    mtls = true
    ```

2. Deploy ArmoniK.

3. Certificates will be generated in `all-in-one/generated/certificates/ingress`:
   - For TLS: `ca.crt`
   - For mTLS: `ca.crt`, `client.submitter.crt`, `client.submitter.key`

4. With these certificates, you will be able to create credentials for a secure channel.

### Modify Hosts File

Update your system's hosts file to associate the ArmoniK control plane address with the domain name "armonik.local". Use the following command to edit the hosts file:

```bash
sudo nano /etc/hosts
```

### Use ArmoniK Endpoint in Python

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
    python examples/python/secure_grpc_client.py --endpoint armonik.local:5001 --ca <ca.crt path>
    ```

3. **For Mutual TLS Secure Channel**

    ```bash
    python examples/python/secure_grpc_client.py --endpoint armonik.local:5001 --ca <ca.crt path> --key <client.submitter.key path> --cert <client.submitter.crt>
    ```
