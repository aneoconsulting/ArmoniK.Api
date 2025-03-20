from __future__ import annotations

import concurrent.futures
import logging
import threading
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
        self._results_client = ArmoniKResults(grpc_channel)
        self._download_metrics: Dict[str, Dict[str, float]] = {}
        self._metrics_lock = threading.Lock()
        self._results_lock = threading.Lock()

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

    def wait_for_availability_and_download(
        self,
        result_ids: Union[str, List[str]],
        session_id: str,
        bucket_size: int = 100,
        parallelism: int = 1,
    ) -> Dict[str, bytes]:
        """
        Waits for results to become COMPLETED and then downloads the result.
        For each completed result, a thread from ThreadPoolExecutor is used to download
        as soon as the notification is received. Metrics about download time are stored.

        Args:
            result_ids: The IDs of the results.
            session_id: The session ID.
            bucket_size: Batch size.
            parallelism: Number of threads to concurrently handle batches.

        Returns:
            Dictionary mapping result IDs to their downloaded data.
        """
        if isinstance(result_ids, str):
            result_ids = [result_ids]
        result_ids = set(result_ids)
        if not result_ids:
            return {}

        downloaded_results: Dict[str, bytes] = {}

        with self._metrics_lock:
            for result_id in result_ids:
                self._download_metrics[result_id] = {"status": "pending"}

        max_workers = max(parallelism * 2, 10)
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            try:
                if parallelism > 1:
                    # Create futures for each batch
                    futures = [
                        executor.submit(
                            _wait_and_download,
                            self,
                            session_id,
                            batch,
                            downloaded_results,
                        )
                        for batch in batched(result_ids, bucket_size)
                    ]

                    # Wait for all futures to complete
                    for future in concurrent.futures.as_completed(futures):
                        exp = future.exception()
                        if exp is not None:
                            for f in futures:
                                f.cancel()
                            raise exp
                else:
                    # Process batches sequentially
                    for batch in batched(result_ids, bucket_size):
                        _wait_and_download(self, session_id, batch, downloaded_results)
            except Exception as e:
                logging.error("Error in wait_for_availability_and_download: %s", e)
                raise

        return downloaded_results

    def download_result(
        self, result_id: str, session_id: str, results_dict: Dict[str, bytes]
    ) -> None:
        """
        Downloads the result for the given result_id and stores it in results_dict.
        Measures the time between download start and completion.
        Records metrics in self._download_metrics.

        Args:
            result_id: The ID of the result to download.
            session_id: The session ID.
            results_dict: Dictionary to store the downloaded result.
        """
        download_start = time.monotonic()

        try:
            result_data = self._results_client.download_result_data(result_id, session_id)

            with self._results_lock:
                results_dict[result_id] = result_data

            download_end = time.monotonic()
            elapsed = download_end - download_start

            with self._metrics_lock:
                self._download_metrics[result_id] = {
                    "start_time": download_start,
                    "end_time": download_end,
                    "elapsed": elapsed,
                    "status": "completed",
                }

            logging.info("Result %s downloaded in %.2f seconds", result_id, elapsed)
        except Exception as e:
            logging.error("Error downloading result %s: %s", result_id, e)
            with self._metrics_lock:
                if result_id not in self._download_metrics:
                    self._download_metrics[result_id] = {}
                self._download_metrics[result_id]["status"] = "failed"
                self._download_metrics[result_id]["error"] = str(e)
            raise

    def get_download_metrics(self) -> Dict[str, Dict[str, float]]:
        """
        Returns metrics about the downloads performed.

        Returns:
            Dictionary mapping result IDs to metrics dictionaries.
        """
        with self._metrics_lock:
            return self._download_metrics.copy()


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


def _wait_and_download(
    event_client: ArmoniKEvents,
    session_id: str,
    results: Collection[str],
    results_dict: Dict[str, bytes],
):
    """
    Waits for results to become available and downloads them as soon as they are ready.

    Args:
        event_client: The ArmoniKEvents client.
        session_id: The session ID.
        results: Collection of result IDs to wait for.
        results_dict: Dictionary to store downloaded results.
    """
    if not results:
        return

    results_filter = _create_results_filter(results)
    not_found = set(results)

    with ThreadPoolExecutor(max_workers=min(len(results), 20)) as download_executor:

        def handler(_, _2, event: Event) -> bool:
            event = cast(Union[NewResultEvent, ResultStatusUpdateEvent], event)
            if event.result_id in not_found:
                if event.status == ResultStatus.COMPLETED:
                    not_found.remove(event.result_id)
                    download_executor.submit(
                        event_client.download_result,
                        event.result_id,
                        session_id,
                        results_dict,
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
                        "Maximum retries (%d) reached when waiting for results: %s",
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


def _wait_all(event_client: ArmoniKEvents, session_id: str, results: Collection[str]):
    """
    Waits for all results to become available.

    Args:
        event_client: The ArmoniKEvents client.
        session_id: The session ID.
        results: Collection of result IDs to wait for.
    """
    if len(results) == 0:
        return

    results_filter = _create_results_filter(results)
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
                    "Maximum retries (%d) reached when waiting for results: %s",
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
