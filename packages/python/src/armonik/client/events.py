from __future__ import annotations

import concurrent.futures
from concurrent.futures.thread import ThreadPoolExecutor
from typing import Callable, cast, Iterable, List, Optional, Union, Collection

from grpc import Channel, RpcError

from ..common import (
    EventTypes,
    NewTaskEvent,
    NewResultEvent,
    ResultOwnerUpdateEvent,
    ResultStatusUpdateEvent,
    TaskStatusUpdateEvent,
    ResultStatus,
    Event,
    Result,
    batched,
)
from ..common.filter import Filter
from ..protogen.client.events_service_pb2_grpc import EventsStub
from ..protogen.common.events_common_pb2 import EventSubscriptionRequest, EventsEnum
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
            tasks_filters=cast(rawTaskFilters, task_filter.to_disjunction().to_message())
            if task_filter is not None
            else rawTaskFilters(),
            results_filters=cast(rawResultFilters, result_filter.to_disjunction().to_message())
            if result_filter is not None
            else rawResultFilters(),
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
