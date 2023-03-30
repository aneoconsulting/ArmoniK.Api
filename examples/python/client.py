#!/usr/bin/env python3
import grpc
import argparse
from armonik.client import ArmoniKSubmitter
from armonik.common import TaskDefinition, TaskOptions
from datetime import timedelta
from common import Payload, Result


def parse_arguments():
    parser = argparse.ArgumentParser("Armonik Example Client")
    parser.add_argument("-e", "--endpoint", required=True, type=str, help="Control plane endpoint")
    parser.add_argument("-p", "--partition", type=str, help="Partition used for the worker")
    parser.add_argument("-v", "--values", type=float, help="List of values to compute instead of x in [0, n[", nargs='+')
    parser.add_argument("-n", "--nfirst", type=int, help="Compute from 0 inclusive to n exclusive, n=10 by default", default=10)
    return parser.parse_args()


def main():
    args = parse_arguments()
    # Open a channel to the control plane
    with grpc.insecure_channel(args.endpoint) as channel:
        # Create a task submitting client
        client = ArmoniKSubmitter(channel)
        # Default task options to be used in a session
        default_task_options = TaskOptions(max_duration=timedelta(seconds=300), priority=1, max_retries=5)
        # Create a session
        session_id = client.create_session(default_task_options=default_task_options, partition_ids=[args.partition] if args.partition is not None else None)
        try:
            # Create the payload
            payload = Payload([i for i in range(args.nfirst)] if args.values is None else args.values)
            # Define the task with the payload
            task_definition = TaskDefinition(payload.serialize(), expected_output_ids=[client.request_output_id(session_id)])
            # Submit the task
            submitted_tasks, submission_errors = client.submit(session_id, [task_definition])
            for e in submission_errors:
                print(f"Submission error : {e}")

            for t in submitted_tasks:
                # Wait for the result to be available
                reply = client.wait_for_availability(session_id, result_id=t.expected_output_ids[0])
                if reply is None:
                    # This should not happen
                    print("Result unexpectedly unavailable")
                    continue
                if reply.is_available():
                    # Result is available, get the result
                    result_payload = Result.deserialize(client.get_result(session_id, result_id=t.expected_output_ids[0]))
                    print(f"Result : {result_payload.value}")
                else:
                    # Result is in error
                    errors = "\n".join(reply.errors)
                    print(f'Errors : {errors}')
        except KeyboardInterrupt:
            # If we stop the script, cancel the session
            client.cancel_session(session_id)


if __name__ == "__main__":
    main()
