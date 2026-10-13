# Connecting to ArmoniK Securely using gRPC in Python

For the general TLS/mTLS setup in Python, see [Authentication](../getting-started/authentication.md#python). This
page covers the certificate mismatch issue that comes up specifically with ArmoniK's default self-signed
certificates.

## Prerequisites

ArmoniK must be deployed with authentication enabled. Follow
[How to configure authentication in ArmoniK](https://armonik.readthedocs.io/en/latest/content/user-guide/how-to-configure-authentication.html)
(ArmoniK docs). That deployment produces:

- `ca.crt` — Certificate Authority root certificate (TLS and mTLS)
- `client.submitter.crt` / `client.submitter.key` — client certificate and key (mTLS only)

## Certificate CN doesn't match the endpoint name

This applies, for example, when using ArmoniK's default certificates: when connecting, the Common Name (CN) of the
server certificate must match the endpoint hostname you connect to — otherwise the TLS handshake fails with a
hostname verification error.

Update your system's hosts file to associate the ArmoniK control plane address with the domain name used in the
certificate's CN (typically `armonik.local`):

```bash
sudo nano /etc/hosts
```

Then use that name as the endpoint, including the port:

```python
from armonik.common import create_channel

channel = create_channel("https://armonik.local:5001", certificate_authority="ca.crt")
```

See [Troubleshooting](../troubleshooting/index.md) for the corresponding `UNAVAILABLE`/handshake error symptoms.
