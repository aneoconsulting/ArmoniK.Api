from copy import deepcopy
from typing import Callable, cast, Iterable, List, Optional

from grpc import Channel

from .results import ArmoniKResults
from ..common import (
    EventTypes,
    NewTaskEvent,
    NewResultEvent,
    ResultOwnerUpdateEvent,
    ResultStatusUpdateEvent,
    TaskStatusUpdateEvent,
    ResultStatus,
    Event,
)
from .results import ResultFieldFilter
from ..common.filter import Filter
from ..protogen.client.events_service_pb2_grpc import EventsStub
from ..protogen.common.events_common_pb2 import EventSubscriptionRequest
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
        self._results_client = ArmoniKResults(grpc_channel)

    def get_events(
        self,
        session_id: str,
        event_types: Iterable[
            EventTypes
        ],  # TODO: make EventTypes an enum when Python 3.8 support will be not supported
        event_handlers: List[Callable[[str, EventTypes, Event], bool]],
        task_filter: Optional[Filter] = None,
        result_filter: Optional[Filter] = None,
    ) -> None:
        """Get events that represents updates of result and tasks data.

        Args:
            session_id: The ID of the session.
            event_types: The list of the types of event to catch.
            event_handlers: The list of handlers that process the events. Handlers are evaluated in he order they are provided.
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
            tasks_filters=cast(rawTaskFilters, task_filter.to_disjunction().to_message()) if task_filter else rawTaskFilters(),
            results_filters=cast(
                rawResultFilters, result_filter.to_disjunction().to_message()
            ) if result_filter else rawResultFilters(),
        )

        streaming_call = self._client.GetEvents(request)
        for message in streaming_call:
            event_type = message.WhichOneof("update")
            for event_handler in event_handlers: 
                if event_handler(
                    session_id,
                    EventTypes.from_string(event_type),
                    self._events_obj_mapping[event_type].from_raw_event(
                        getattr(message, event_type)
                    ),
                ):
                    break

    def wait_for_result_availability(self, result_ids: List[str], session_id: str) -> None:
        """Wait until a result is ready i.e its status updates to COMPLETED.

        Args:
            result_ids: The IDs of the results.
            session_id: The ID of the session.

        Raises:
            RuntimeError: If the result status is ABORTED.
        """

        results_not_found = deepcopy(result_ids)

        results_filter = (ResultFieldFilter.RESULT_ID == result_ids[0])
        for result_id in result_ids[1:]:
            results_filter = results_filter | (ResultFieldFilter.RESULT_ID == result_id)

        request = EventSubscriptionRequest(
            session_id=session_id,
            returned_events=[EventTypes.RESULT_STATUS_UPDATE],
            tasks_filters=rawTaskFilters(),
            results_filters=cast(
                rawResultFilters, results_filter.to_disjunction().to_message()
            ),
        )

        while results_not_found:
            streaming_call = self._client.GetEvents(request)
            try:
                for message in streaming_call:
                    event_type = EventTypes.from_string(message.WhichOneof("update"))
                    if event_type == EventTypes.RESULT_STATUS_UPDATE:
                        event = ResultStatusUpdateEvent.from_raw_event(message.result_status_update)
                        if event.result_id in results_not_found:
                            if event.status == ResultStatus.COMPLETED:
                                results_not_found.remove(event.result_id)
                                if not results_not_found:
                                    break
                            if event.status == ResultStatus.ABORTED:
                                raise RuntimeError(f"Result {event.result_id} has been aborted.")
                    if event_type == EventTypes.NEW_RESULT:
                        event = NewResultEvent.from_raw_event(messae.new_result)
                        if event.result_id in results_not_found:
                            if event.status == ResultStatus.COMPLETED:
                                results_not_found.remove(event.result_id)
                                if not results_not_found:
                                    break
                            if event.status == ResultStatus.ABORTED:
                                RuntimeError(f"Result {event.result_id} has been aborted.")

            except grpc.RpcError:
                pass
