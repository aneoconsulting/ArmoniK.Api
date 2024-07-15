#!/usr/bin/env python3
import grpc
import argparse
from typing import cast
from armonik.client import ArmoniKResults, ArmoniKTasks, ArmoniKSessions, ArmoniKEvents
from armonik.client.tasks import TaskFieldFilter
from armonik.common import TaskDefinition, TaskOptions
from datetime import timedelta, datetime
from common import Payload, Result, InputPayload


def parse_arguments():
    parser = argparse.ArgumentParser("ArmoniK Example Client")
    parser.add_argument("-e", "--endpoint", required=True, type=str, help="Control plane endpoint")
    parser.add_argument("-p", "--partition", type=str, help="Partition used for the worker")
    parser.add_argument("-v", "--values", type=float, help="List of values to compute instead of x in [0, n[", nargs='+')
    parser.add_argument("-n", "--nfirst", type=int, help="Compute from 0 inclusive to n exclusive, n=10 by default", default=10)
    parser.add_argument("-l", "--list", action="store_true", help="List tasks of the session at the end")
    return parser.parse_args()


def main():
    args = parse_arguments()
    print("Hello ArmoniK Python Example !")
    # Open a channel to the control plane
    with grpc.insecure_channel(args.endpoint) as channel:
        # Create a task submitting client
        tasks_client = ArmoniKTasks(channel)
        # Create the results client
        results_client = ArmoniKResults(channel)
        # Create the session client
        session_client = ArmoniKSessions(channel)
        # Default task options to be used in a session
        default_task_options = TaskOptions(max_duration=timedelta(seconds=300), priority=1, max_retries=5, partition_id=args.partition)
        # Create a session
        session_id = session_client.create_session(default_task_options=default_task_options, partition_ids=[args.partition] if args.partition is not None else None)
        print(f"Session {session_id} has been created")
        try:
            # Create the payload
            payload = Payload([i for i in range(args.nfirst)] if args.values is None else args.values)
            # Create the result and payload
            result_name = f"main_result_{int(datetime.now().timestamp())}"
            payload_name = "payload_name"
            results = results_client.create_results_metadata([result_name, payload_name], session_id)
            # Define the task with the payload
            task_definition = TaskDefinition(payload_id=results[payload_name].result_id, payload=b'', expected_output_ids=[results[result_name].result_id])
            # Upload payload
            results_client.upload_result_data(results[payload_name].result_id, session_id, payload.serialize())
            # Submit the task
            tasks_client.submit_tasks(session_id, [task_definition])
            print("Main task has been sent")

            event_client = ArmoniKEvents(channel)

            try:
                event_client.wait_for_result_availability(results["result_name"].result_id, session_id)
                Result.deserialize(results_client.download_result_data(results["result_name"].result_id, session_id))
            except RuntimeError as e:
                print(f"Error: {e}")

            # for t in submitted_tasks:
            #     # Wait for the result to be available
            #     reply = client.wait_for_availability(session_id, result_id=t.expected_output_ids[0])
            #     if reply is None:
            #         # This should not happen
            #         print("Result unexpectedly unavailable")
            #         continue
            #     if reply.is_available():
            #         # Result is available, get the result
            #         result_payload = Result.deserialize(cast(bytes, client.get_result(session_id, result_id=t.expected_output_ids[0])))
            #         print(f"Result : {result_payload.value}")
            #     else:
            #         # Result is in error
            #         errors = "\n".join(reply.errors)
            #         print(f'Errors : {errors}')

            # List tasks
            if args.list:
                print(f"Listing tasks of session {session_id}")
                # Create the tasks client
                tasks_client = ArmoniKTasks(channel)

                # Request listing of tasks from the session
                total_tasks, tasks = tasks_client.list_tasks(TaskFieldFilter.SESSION_ID == session_id)
                print(f"Found {total_tasks} tasks in total for the session {session_id}")

                for t in tasks:
                    print(t)

        except KeyboardInterrupt:
            # If we stop the script, cancel the session
            session_client.cancel_session(session_id)
            print("Session has been cancelled")
        finally:
            print("Good bye !")


if __name__ == "__main__":
    main()
