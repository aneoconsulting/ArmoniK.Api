import os
from os import PathLike
from typing import Union, Tuple, Optional
from urllib.parse import urlparse

from grpc import Channel, secure_channel, insecure_channel, ssl_channel_credentials
from cryptography.hazmat.primitives.serialization import (
    pkcs12,
    Encoding,
    PrivateFormat,
    NoEncryption,
)


def _read_file(path: Union[str, PathLike]) -> bytes:
    with open(path, "rb") as f:
        return f.read()


_ca_cert_locations = [
    "/etc/ssl/certs/ca-certificates.crt",  # Debian/Ubuntu/Gentoo etc.
    "/etc/pki/tls/certs/ca-bundle.crt",  # Fedora/RHEL 6
    "/etc/ssl/ca-bundle.pem",  # OpenSUSE
    "/etc/pki/tls/cacert.pem",  # OpenELEC
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",  # CentOS/RHEL 7
    "/etc/ssl/cert.pem",  # Alpine Linux
]

_cached_ca_cert_location: Optional[str] = ""


def _find_bundle_path() -> Optional[str]:
    global _cached_ca_cert_location
    if _cached_ca_cert_location is not None and len(_cached_ca_cert_location) > 0:
        return _cached_ca_cert_location
    # Look if one exists
    for location in _ca_cert_locations:
        if os.path.exists(location):
            _cached_ca_cert_location = location
            return location
    # Not found, will use the default provided
    _cached_ca_cert_location = None


def _load_certificates(
    certificate_authority: Union[str, PathLike, bytes, None] = None,
    client_certificate: Union[str, PathLike, bytes, None] = None,
    client_key: Union[str, PathLike, bytes, None] = None,
) -> Tuple[bytes, bytes, bytes]:
    if certificate_authority is None:
        certificate_authority = _find_bundle_path()  # Otherwise it uses the ca bundle of certifi. We want to find the installed ca bundle instead

    if certificate_authority is not None:
        if not isinstance(certificate_authority, bytes):
            certificate_authority = _read_file(certificate_authority)

    if client_certificate is not None:
        if not isinstance(client_certificate, bytes):
            client_certificate = _read_file(client_certificate)
        if client_key is None:
            # client and key are in the same file
            try:
                # Try to parse p12 if it's a p12
                pfx = pkcs12.load_pkcs12(client_certificate, b"")
                client_certificate = pfx.cert.certificate.public_bytes(Encoding.PEM)
                client_key = pfx.key.private_bytes(
                    Encoding.PEM, PrivateFormat.TraditionalOpenSSL, NoEncryption()
                )
            except ValueError:
                # Probably a PEM file
                client_key = client_certificate
        else:
            if not isinstance(client_key, bytes):
                client_key = _read_file(client_key)

    return certificate_authority, client_certificate, client_key


def create_channel(
    uri: str,
    *,
    options: Union[Tuple[Tuple[str, str]]] = None,
    certificate_authority: Union[str, PathLike, bytes, None] = None,
    client_certificate: Union[str, PathLike, bytes, None] = None,
    client_key: Union[str, PathLike, bytes, None] = None,
) -> Channel:
    """
    Create a gRPC channel for communication with the ArmoniK control plane
    Args:
        uri: URI of the channel. Will start a secure channel if the scheme contains "https". If it contains "unix", uses a unix socket.
        options: Options to pass to the channel
        certificate_authority: Certificate authority path to read, or content as bytes
        client_certificate: Client certificate path to read, or content as bytes
        client_key: Client key path to read, or content as bytes. If set to None but client_certificate is not None, assumes the key is included with the certificate (p12 or PEM certificate)
    Returns:
        Channel: gRPC channel for communication
    """
    parsed = urlparse(uri)
    scheme = parsed.scheme if parsed.scheme != "" else "http"
    endpoint = (
        parsed.netloc + parsed.path
    )  # To support with or without scheme, and for paths for unix

    if "unix" in scheme:
        # gRPC supports unix:path and the path is then relative, if the scheme is unix://, then the path is absolute
        if endpoint.startswith("/"):
            endpoint = "unix://" + endpoint
        else:
            endpoint = "unix:" + endpoint

    if "https" in scheme:
        certificate_authority, client_certificate, client_key = _load_certificates(
            certificate_authority, client_certificate, client_key
        )

        return secure_channel(
            endpoint,
            ssl_channel_credentials(
                root_certificates=certificate_authority,
                private_key=client_key,
                certificate_chain=client_certificate,
            ),
            options=options,
        )
    else:
        return insecure_channel(endpoint, options=options)
