import logging
import os

import grpc
from armonik.worker import ArmoniKWorker, TaskHandler, ClefLogger
from armonik.common import Output, TaskDefinition
from typing import List, Union, cast

from common import Payload, Result

ClefLogger.setup_logging(logging.INFO)


# Task processing
def processor(task_handler: TaskHandler) -> Output:
    logger = ClefLogger.getLogger("ArmoniKWorker")
    payload = Payload.deserialize(task_handler.payload)
    # No values
    if len(payload.values) == 0:
        if task_handler.expected_results:
            task_handler.send_result(task_handler.expected_results[0], Result(0.0).serialize())
        logger.info("No values")
        return Output()

    if isinstance(payload.values[0], str):
        # Aggregation task
        results = [Result.deserialize(task_handler.data_dependencies[r]).value for r in cast(List[str], payload.values)]
        task_handler.send_result(task_handler.expected_results[0], Result(aggregate(results)).serialize())
        logger.info(f"Aggregated {len(results)} values")
        return Output()

    if len(payload.values) <= 1 or len(payload.values) <= payload.subtask_threshold:
        # Compute
        task_handler.send_result(task_handler.expected_results[0], Result(aggregate(cast(List[float], payload.values))).serialize())
        logger.info(f"Computed {len(payload.values)} values")
        return Output()

    # Subtasking
    pivot = len(payload.values) // 2
    # Split payload in half
    lower = payload.values[:pivot]
    upper = payload.values[pivot:]
    # Create sub-results
    subresults = task_handler.get_results_ids([f"{task_handler.task_id}_lower", f"{task_handler.task_id}_upper"])
    subtasks = []
    for result_id, vals in [(subresults[f"{task_handler.task_id}_lower"], lower), (subresults[f"{task_handler.task_id}_upper"],upper)]:
        # Create new payloads and task definitions
        new_payload = Payload(values=vals, subtask_threshold=payload.subtask_threshold).serialize()
        subtasks.append(TaskDefinition(payload=new_payload, expected_output_ids=[result_id]))
    # Create the aggregation task
    aggregate_dependencies = [s.expected_output_ids[0] for s in subtasks]
    subtasks.append(TaskDefinition(Payload(values=aggregate_dependencies).serialize(), expected_output_ids=task_handler.expected_results, data_dependencies=aggregate_dependencies))

    # Submit tasks
    submitted, errors = task_handler.create_tasks(subtasks)
    if len(errors) > 0:
        message = f"Errors while submitting subtasks : {', '.join(errors)}"
        logger.error(message)
        return Output(message)
    logger.info(f"Submitted {len(submitted)} subtasks")
    return Output()


def aggregate(values: List[Union[int, float]]) -> float:
    return sum(values)


def main():
    # Create Seq compatible logger
    logger = ClefLogger.getLogger("ArmoniKWorker")
    # Define agent-worker communication endpoints
    worker_scheme = "unix://" if os.getenv("ComputePlane__WorkerChannel__SocketType", "unixdomainsocket") == "unixdomainsocket" else "http://"
    agent_scheme = "unix://" if os.getenv("ComputePlane__AgentChannel__SocketType", "unixdomainsocket") == "unixdomainsocket" else "http://"
    worker_endpoint = worker_scheme+os.getenv("ComputePlane__WorkerChannel__Address", "/cache/armonik_worker.sock")
    agent_endpoint = agent_scheme+os.getenv("ComputePlane__AgentChannel__Address", "/cache/armonik_agent.sock")

    # Start worker
    logger.info("Worker Started")
    with grpc.insecure_channel(agent_endpoint) as agent_channel:
        worker = ArmoniKWorker(agent_channel, processor, logger=logger)
        logger.info("Worker Connected")
        worker.start(worker_endpoint)


if __name__ == "__main__":
    main()
