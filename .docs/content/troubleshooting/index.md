# Troubleshooting

This page collects common issues and their solutions when working with ArmoniK.Api.

## gRPC connection refused

**Symptom**: `StatusCode.UNAVAILABLE` or `Connection refused` when connecting to ArmoniK.

**Check**:
- ArmoniK is deployed and the control plane is reachable at the configured endpoint.
- The endpoint includes the port (e.g. `armonik.local:5001`).
- `GrpcClient__AllowUnsafeConnection` is set correctly for your deployment (TLS vs plain).

## Certificate CN mismatch

**Symptom**: TLS handshake fails with a hostname verification error.

**Fix**: Either set `GrpcClient__OverrideTargetName` to the CN in the certificate, or add the CN to your `/etc/hosts` file and use it as the endpoint. See [Connecting securely with Python](../usage/use-armonik-grpc-securely-python.md).

## Sphinx build fails locally

**Symptom**: Sphinx errors about missing files or modules.

**Check**:
- Run all generation scripts before `sphinx-build` (see [README](../../README.md)).
- The Python virtual environment is activated and `requirements.txt` is installed.
- Doxygen XML is present under `content/api/cpp/doxygen/xml/`.
