#!/usr/bin/env python3
import grpc
import argparse
from typing import cast
from armonik.client import ArmoniKSessions, ArmoniKResults, ArmoniKTasks, ArmoniKEvents
from armonik.client.tasks import TaskFieldFilter
from armonik.client.results import ResultFieldFilter
from armonik.common import TaskDefinition, TaskOptions, Result
from datetime import timedelta, datetime
from common import TaskIO


def parse_arguments():
    parser = argparse.ArgumentParser("ArmoniK Example Client")
    parser.add_argument("-e", "--endpoint", required=True, type=str, help="Control plane endpoint")
    parser.add_argument("-p", "--partition", type=str, help="Partition used for the worker")
    parser.add_argument("-v", "--values", type=float, help="List of values to compute instead of x in [0, n[", nargs='+')
    parser.add_argument("-n", "--nfirst", type=int, help="Compute from 0 inclusive to n exclusive, n=10 by default", default=10)
    parser.add_argument("-l", "--list", action="store_true", help="List tasks of the session at the end")
    parser.add_argument('--threshold', type=str, help='value of threshold', default="30")
    return parser.parse_args()

def main():
    args = parse_arguments()
    print("Hello ArmoniK Python Example!")
    # Create User Data converted in bytes in common
    user_data = TaskIO([1, 2, 3])
    # Open a channel to the control plane
    with grpc.insecure_channel(args.endpoint) as channel:
        # Create the task client
        task_client = ArmoniKTasks(channel)
        # Create the result client
        result_client = ArmoniKResults(channel)
        # Create the session client
        session_client = ArmoniKSessions(channel)
        # Default task options to be used in a session
        default_task_options = TaskOptions(max_duration=timedelta(seconds=300), priority=1, max_retries=5, partition_id=args.partition)
        # Create a session
        session_id = session_client.create_session(default_task_options=default_task_options, partition_ids=[args.partition] if args.partition is not None else None)
        print(f"Session {session_id} has been created")
        # Create Result input metadata
        results = result_client.create_results_metadata(["input","output"], session_id)
        # Submit data for an empty result already created
        result_client.upload_result_data(results["input"].result_id, session_id, user_data.serialize())
        # Create Task definition
        task_definition = TaskDefinition(
            expected_output_ids=[results["output"].result_id],
            data_dependencies=[results["input"].result_id],
            options=TaskOptions(
                max_duration=timedelta(seconds=300),
                priority=1,
                max_retries=5,
                options={"threshold": args.threshold}
            )
        )
        print(task_definition)
        # Submit Task
        submitted_task = task_client.submit_tasks(session_id, [task_definition])

        # Wait for the result to be available

        # Downlaod output data of result


if __name__ == "__main__":
    main()