from datetime import timedelta
import time
from threading import Thread

import grpc
import pytest
from armonik.client import ArmoniKTasks, ArmoniKResults, ArmoniKSessions, ArmoniKEvents
from armonik.common import TaskOptions, TaskDefinition

endpoint = ""


def wait_and_unpause(session_id: str):
    time.sleep(1)
    with grpc.insecure_channel(endpoint) as channel:
        ArmoniKSessions(channel).resume_session(session_id)
        print("Session resumed")


class TestWaitAvailability:
    def test_wait_availability(self):
        pytest.skip()
        n_tasks = 10000
        with grpc.insecure_channel(endpoint) as channel:
            task_client = ArmoniKTasks(channel)
            result_client = ArmoniKResults(channel)
            session_client = ArmoniKSessions(channel)
            events_client = ArmoniKEvents(channel)
            session_id = session_client.create_session(TaskOptions(timedelta(seconds=60), 1, 1, ""))
            print(f"Created session {session_id}")
            session_client.pause_session(session_id)
            payload_ids = list(
                r.result_id
                for r in result_client.create_results(
                    {str(r): str(r).encode() for r in range(n_tasks)}, session_id
                ).values()
            )
            print(f"Submitted payloads {len(payload_ids)}")
            result_ids = list(
                r.result_id
                for r in result_client.create_results_metadata(
                    [str(r) for r in range(n_tasks)], session_id
                ).values()
            )
            print(f"Submitted results {len(result_ids)}")
            tasks = task_client.submit_tasks(
                session_id,
                [
                    TaskDefinition(payload_id=p, expected_output_ids=[r])
                    for p, r in zip(payload_ids, result_ids)
                ],
            )
            print(f"Submitted tasks {len(tasks)}")
            t = Thread(target=wait_and_unpause, args=(session_id,))
            start = time.time()
            t.start()
            print("Waiting on results")
            events_client.wait_for_result_availability(result_ids, session_id, bucket_size=100)
            end = time.time()
            print(end - start)
            session_client.close_session(session_id)
            session_client.purge_session(session_id)
            session_client.delete_session(session_id)

    def test_wait_availability2(self):
        pytest.skip()
        n_tasks = 10000
        with grpc.insecure_channel(endpoint) as channel:
            task_client = ArmoniKTasks(channel)
            result_client = ArmoniKResults(channel)
            session_client = ArmoniKSessions(channel)
            events_client = ArmoniKEvents(channel)
            session_id = session_client.create_session(TaskOptions(timedelta(seconds=60), 1, 1, ""))
            print(f"Created session {session_id}")
            session_client.pause_session(session_id)
            payload_ids = list(
                r.result_id
                for r in result_client.create_results(
                    {str(r): str(r).encode() for r in range(n_tasks)}, session_id
                ).values()
            )
            print(f"Submitted payloads {len(payload_ids)}")
            result_ids = list(
                r.result_id
                for r in result_client.create_results_metadata(
                    [str(r) for r in range(n_tasks)], session_id
                ).values()
            )
            print(f"Submitted results {len(result_ids)}")
            tasks = task_client.submit_tasks(
                session_id,
                [
                    TaskDefinition(payload_id=p, expected_output_ids=[r])
                    for p, r in zip(payload_ids, result_ids)
                ],
            )
            print(f"Submitted tasks {len(tasks)}")
            t = Thread(target=wait_and_unpause, args=(session_id,))
            start = time.time()
            t.start()
            print("Waiting on results")
            events_client.wait_for_result_availability(
                result_ids, session_id, bucket_size=100, parallelism=10
            )
            end = time.time()
            print(end - start)
            session_client.close_session(session_id)
            session_client.purge_session(session_id)
            session_client.delete_session(session_id)
