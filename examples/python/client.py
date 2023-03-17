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
    with grpc.insecure_channel(args.endpoint) as channel:
        client = ArmoniKSubmitter(channel)
        default_task_options = TaskOptions(max_duration=timedelta(seconds=300), priority=1, max_retries=5)
        session_id = client.create_session(default_task_options=default_task_options, partition_ids=[args.partition] if args.partition is not None else None)
        try:
            payload = Payload([i for i in range(args.nfirst)] if args.values is None else args.values)
            task_definition = TaskDefinition(payload.serialize(), expected_output_ids=[client.request_output_id(session_id)])
            submitted_tasks, submission_errors = client.submit(session_id, [task_definition])
            for e in submission_errors:
                print(f"Submission error : {e}")
            if len(submitted_tasks) > 0:
                for t in submitted_tasks:
                    reply = client.wait_for_availability(session_id, result_id=t.expected_output_ids[0])
                    if reply is None:
                        print("Result unexpectedly unavailable")
                        continue
                    if reply.is_available():
                        result_payload = Result.deserialize(client.get_result(session_id, result_id=t.expected_output_ids[0]))
                        print(f"Result : {result_payload.value}")
                    else:
                        errors = "\n".join(reply.errors)
                        print(f'Errors : {errors}')
        except KeyboardInterrupt:
            client.cancel_session(session_id)


if __name__ == "__main__":
    main()
