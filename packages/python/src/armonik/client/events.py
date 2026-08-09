from __future__ import annotations

import concurrent.futures
import logging
import time
from concurrent.futures.thread import ThreadPoolExecutor
from typing import Callable, Collection, Dict, Iterable, List, Optional, Union, cast

from grpc import Channel, RpcError

from ..client.results import ArmoniKResults
from ..common import (
    Event,
    EventTypes,
    NewResultEvent,
    NewTaskEvent,
    Result,
    ResultOwnerUpdateEvent,
    ResultStatus,
    ResultStatusUpdateEvent,
    TaskStatusUpdateEvent,
    batched,
)
from ..common.filter import Filter
from ..protogen.client.events_service_pb2_grpc import EventsStub
from ..protogen.common.events_common_pb2 import EventsEnum, EventSubscriptionRequest
from ..protogen.common.results_filters_pb2 import Filters as rawResultFilters
from ..protogen.common.tasks_filters_pb2 import Filters as rawTaskFilters


class ArmoniKEvents:
    _events_obj_mapping = {
        "new_result": NewResultEvent,
        "new_task": NewTaskEvent,
        "result_owner_update": ResultOwnerUpdateEvent,
        "result_status_update": ResultStatusUpdateEvent,
        "task_status_update": TaskStatusUpdateEvent,
    }

    def __init__(self, grpc_channel: Channel):
        """Events service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = EventsStub(grpc_channel)
        self._channel = grpc_channel
        self.results_client = ArmoniKResults(grpc_channel)

    def get_events(
        self,
        session_id: str,
        event_types: Iterable[
            EventsEnum
        ],  # TODO: make EventTypes an enum when Python 3.8 support will be not supported
        event_handlers: List[Callable[[str, EventTypes, Event], bool]],
        task_filter: Optional[Filter] = None,
        result_filter: Optional[Filter] = None,
    ) -> None:
        """Get events that represents updates of result and tasks data.

        Args:
            session_id: The ID of the session.
            event_types: The list of the types of event to catch.
            event_handlers: The list of handlers that process the events. Handlers are evaluated in the order they are provided.
                An handler takes three positional arguments: the ID of the session, the type of event and the event as an object.
                An handler returns a boolean, if False the process continues, otherwise the stream is closed and the service stops
                listening to new events. As handlers are evaluated in order, when a handler interrupts execution (by returning True)
                all the handlers following it will not be executed.
            task_filter: A filter on tasks.
            result_filter: A filter on results.

        """
        request = EventSubscriptionRequest(
            session_id=session_id,
            returned_events=event_types,
            tasks_filters=(
                cast(rawTaskFilters, task_filter.to_disjunction().to_message())
                if task_filter is not None
                else rawTaskFilters()
            ),
            results_filters=(
                cast(rawResultFilters, result_filter.to_disjunction().to_message())
                if result_filter is not None
                else rawResultFilters()
            ),
        )

        streaming_call = self._client.GetEvents(request)
        for message in streaming_call:
            event_type = message.WhichOneof("update")
            if any(
                event_handler(
                    session_id,
                    EventTypes.from_string(event_type),
                    self._events_obj_mapping[event_type].from_raw_event(
                        getattr(message, event_type)
                    ),
                )
                for event_handler in event_handlers
            ):
                break

    def wait_for_result_availability(
        self,
        result_ids: Union[str, List[str]],
        session_id: str,
        bucket_size: int = 100,
        parallelism: int = 1,
    ) -> None:
        """Wait until a result is ready i.e its status updates to COMPLETED.

        Args:
            result_ids: The IDs of the results.
            session_id: The ID of the session.
            bucket_size: Batch size
            parallelism: Parallelism
        Raises:
            RuntimeError: If the result status is ABORTED.
        """
        if isinstance(result_ids, str):
            result_ids = [result_ids]
        result_ids = set(result_ids)
        if len(result_ids) == 0:
            return

        if parallelism > 1:
            pool = ThreadPoolExecutor(max_workers=parallelism)
            try:
                futures = [
                    pool.submit(_wait_all, self, session_id, batch)
                    for batch in batched(result_ids, bucket_size)
                ]
                for i, future in enumerate(concurrent.futures.as_completed(futures)):
                    exp = future.exception()
                    if exp is not None:
                        for f in futures:
                            f.cancel()
                        raise exp
            finally:
                pool.shutdown(wait=False)
        else:
            for batch in batched(result_ids, bucket_size):
                _wait_all(self, session_id, batch)

    def wait_for_result_availability_and_download(
        self,
        result_ids: Union[str, List[str]],
        session_id: str,
        bucket_size: int = 100,
        parallelism: int = 1,
        submission_timestamp: Optional[float] = None,
    ) -> Dict[str, Dict[str, Union[bytes, float]]]:
        """
        Wait for results to become available and download them immediately.
        This method waits for results to reach COMPLETED status and downloads them
        as soon as they become available without waiting for all results to finish.

        Args:
            result_ids: The IDs of the results.
            session_id: The ID of the session.
            bucket_size: Number of results to process in a single batch.
            parallelism: Number of parallel threads to use.
            submission_timestamp: Optional timestamp when tasks were submitted (time.time()).

        Returns:
            Dictionary mapping result IDs to dictionaries containing:
            - 'data': The downloaded result bytes
            - 'processing_time': Time from submission to completion (if submission_timestamp provided)
            - 'download_time': Time to download the result
            - 'completion_timestamp': When the result became available

        Raises:
            RuntimeError: If a result status is ABORTED.
        """
        # Same initialization code as before
        if isinstance(result_ids, str):
            result_ids = [result_ids]
        result_ids = set(result_ids)
        if len(result_ids) == 0:
            return {}

        if parallelism > 1:
            results_list = list(result_ids)
            chunk_size = max(1, len(results_list) // parallelism)
            result_chunks = [
                results_list[i : i + chunk_size] for i in range(0, len(results_list), chunk_size)
            ]

            all_results = {}
            with ThreadPoolExecutor(max_workers=parallelism) as executor:
                futures = {
                    executor.submit(
                        _wait_and_download_results_with_timing,
                        self,
                        session_id,
                        chunk,
                        submission_timestamp,
                    ): i
                    for i, chunk in enumerate(result_chunks)
                }

                for future in concurrent.futures.as_completed(futures):
                    try:
                        batch_results = future.result()
                        all_results.update(batch_results)
                    except Exception as e:
                        logging.error("Error in wait_for_result_availability_and_download: %s", e)
                        for f in futures:
                            if not f.done():
                                f.cancel()
                        raise e

            return all_results
        else:
            all_results = {}
            for batch in batched(result_ids, bucket_size):
                batch_results = _wait_and_download_results_with_timing(
                    self, session_id, batch, submission_timestamp
                )
                all_results.update(batch_results)

            return all_results


def _wait_all(event_client: ArmoniKEvents, session_id: str, results: Collection[str]):
    if len(results) == 0:
        return
    results_filter = None
    for result_id in results:
        results_filter = (
            Result.result_id == result_id
            if results_filter is None
            else (results_filter | (Result.result_id == result_id))
        )

    not_found = set(results)

    def handler(_, _2, event: Event) -> bool:
        event = cast(Union[NewResultEvent, ResultStatusUpdateEvent], event)
        if event.result_id in not_found:
            if event.status == ResultStatus.COMPLETED:
                not_found.remove(event.result_id)
                if not not_found:
                    return True
            elif event.status == ResultStatus.ABORTED:
                raise RuntimeError(f"Result {event.result_id} has been aborted.")
        return False

    while not_found:
        try:
            event_client.get_events(
                session_id,
                [EventTypes.RESULT_STATUS_UPDATE, EventTypes.NEW_RESULT],
                [handler],
                None,
                results_filter,
            )
        except RpcError:
            pass
        else:
            break


def _wait_and_download_results_with_timing(
    event_client: ArmoniKEvents,
    session_id: str,
    results: Collection[str],
    submission_timestamp: Optional[float] = None,
) -> Dict[str, Dict[str, Union[bytes, float]]]:
    """
    Wait for results to become available and download them with timing information.
    Args:
        event_client: The ArmoniKEvents client.
        session_id: The session ID.
        results: Collection of result IDs to wait for.
        submission_timestamp: Optional timestamp when tasks were submitted (time.time()).

    Returns:
        Dictionary mapping result IDs to dictionaries containing:
          - 'data': The downloaded result bytes
          - 'processing_time': Time from submission to completion (if submission_timestamp provided)
          - 'download_time': Time to download the result (duration, not timestamp)
          - 'completion_timestamp': When the result became available
          - 'download_complete_timestamp': When the download completed
    """
    if not results:
        return {}

    results_filter = _create_results_filter(results)
    not_found = set(results)
    downloaded_results = {}

    with ThreadPoolExecutor(max_workers=min(len(results), 20)) as download_executor:
        download_futures = {}
        completion_timestamps = {}

        def download_result(result_id, completion_time):
            try:
                download_start_time = time.time()

                result_data = event_client.results_client.download_result_data(
                    result_id, session_id
                )

                download_complete_time = time.time()

                download_duration = download_complete_time - download_start_time

                result_info = {
                    "data": result_data,
                    "download_time": download_duration,
                    "completion_timestamp": completion_time,
                    "download_complete_timestamp": download_complete_time,
                }

                if submission_timestamp is not None:
                    processing_time = completion_time - submission_timestamp
                    result_info["processing_time"] = processing_time

                return result_id, result_info
            except Exception as e:
                logging.error("Failed to download result %s: %s", result_id, e)
                raise

        def handler(_, _2, event: Event) -> bool:
            event = cast(Union[NewResultEvent, ResultStatusUpdateEvent], event)
            if event.result_id in not_found:
                if event.status == ResultStatus.COMPLETED:
                    not_found.remove(event.result_id)
                    completion_time = time.time()
                    completion_timestamps[event.result_id] = completion_time
                    download_futures[event.result_id] = download_executor.submit(
                        download_result, event.result_id, completion_time
                    )
                    if not not_found:
                        return True
                elif event.status == ResultStatus.ABORTED:
                    raise RuntimeError(f"Result {event.result_id} has been aborted.")
            return False

        retry_count = 0
        max_retries = 5
        while not_found:
            try:
                event_client.get_events(
                    session_id,
                    [EventTypes.RESULT_STATUS_UPDATE, EventTypes.NEW_RESULT],
                    [handler],
                    None,
                    results_filter,
                )
            except RpcError as e:
                retry_count += 1
                if retry_count > max_retries:
                    logging.error(
                        "Maximum retries (%d) reached waiting for results: %s",
                        max_retries,
                        e,
                    )
                    raise
                logging.warning(
                    "RPC error while waiting for results (retry %d/%d): %s",
                    retry_count,
                    max_retries,
                    e,
                )
            else:
                break

        for result_id, future in download_futures.items():
            try:
                rid, result_info = future.result()
                downloaded_results[rid] = result_info
            except Exception:
                for f in download_futures.values():
                    if not f.done():
                        f.cancel()
                raise

    return downloaded_results


def _create_results_filter(results: Collection[str]):
    """Helper function to create a filter for a collection of result IDs."""
    if not results:
        return None

    results_filter = None
    for result_id in results:
        results_filter = (
            Result.result_id == result_id
            if results_filter is None
            else (results_filter | (Result.result_id == result_id))
        )
    return results_filter
