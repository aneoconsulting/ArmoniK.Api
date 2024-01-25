import grpc
import argparse
from armonik.client.sessions import ArmoniKSessions, SessionFieldFilter
from armonik.common.enumwrapper import SESSION_STATUS_CANCELLED

def parse_arguments():
    parser = argparse.ArgumentParser(description="ArmoniK Example for Client connection TLS or mutual TLS")
    parser.add_argument("-v","--version", action="version", version="ArmoniK Admin CLI 0.0.1")
    parser.add_argument("--endpoint", default="localhost:5001", help="ArmoniK control plane endpoint")
    parser.add_argument("--ssl", help="Use this option to enable TLS for a secure channel.", action="store_true")
    parser.add_argument("--ca", help="ca.crt path for TLS or mutual TLS")
    parser.add_argument("--cert", help="client certificate path for mutual TLS")
    parser.add_argument("--key", help="client key path for mutual TLS")
    return parser.parse_args()

def read_file(file_path: str) -> bytes:
    with open(file_path, 'rb') as file:
        return file.read()

def create_channel(endpoint: str, ssl: bool, ca: str, key: str, cert: str) -> grpc.Channel:
    """
    Create a gRPC channel for communication with the ArmoniK control plane

    Args:
        ca (str): CA file path for TLS or mutual TLS
        cert (str): Certificate file path for mutual TLS
        key (str): Private key file path for mutual TLS
        endpoint (str): ArmoniK control plane endpoint

    Returns:
        grpc.Channel: gRPC channel for communication
    """
    if ssl:
        ca_data = read_file(ca) if ca else None
        if cert and key:
            cert_data = read_file(cert) if cert else None
            key_data = read_file(key) if key else None
            credentials = grpc.ssl_channel_credentials(root_certificates=ca_data, private_key=key_data, certificate_chain=cert_data)
            print("Hello ArmoniK Python Example Using Mutual TLS !")
        else:
            credentials = grpc.ssl_channel_credentials(root_certificates=ca_data)
            print("Hello ArmoniK Python Example Using TLS !")
        return grpc.secure_channel(endpoint, credentials)
    else:
        print("Hello ArmoniK Python Example using Insecure Channel!")
        return grpc.insecure_channel(endpoint)


def main():
    args = parse_arguments()
    # Open a channel to the control plane
    channel = create_channel(args.endpoint, args.ssl, args.ca, args.key, args.cert)
    # Create the session client
    client = ArmoniKSessions(channel)
    # List numbers sessions with a cancelled status filter
    sessions = client.list_sessions(SessionFieldFilter.STATUS == SESSION_STATUS_CANCELLED)
        
    print(f'\nNumber of sessions: {sessions[0]}\n')


if __name__ == "__main__":
    main()
