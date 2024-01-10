from typing import Callable, cast, List

from grpc import Channel

from .results import ArmoniKResults
from ..common import (
    EventTypes,
    Filter,
    NewTaskEvent,
    NewResultEvent,
    ResultOwnerUpdateEvent,
    ResultStatusUpdateEvent,
    TaskStatusUpdateEvent,
    ResultStatus,
    Event,
)
from .results import ResultFieldFilter
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
        event_types: List[EventTypes],
        event_handlers: List[Callable[[str, EventTypes, Event], bool]],
        task_filter: Filter | None = None,
        result_filter: Filter | None = None,
    ) -> None:
        """Get events that represents updates of result and tasks data.

        Args:
            session_id: The ID of the session.
            event_types: The list of the types of event to catch.
            event_handlers: The list of handlers that process the events. Handlers are evaluated in he order they are provided.
                An handler takes three positional arguments: the ID of the session, the type of event and the event as an object.
                An handler returns a boolean, if True the process continues, otherwise the stream is closed and the service stops
                listening to new events.
            task_filter: A filter on tasks.
            result_filter: A filter on results.

        """
        request = EventSubscriptionRequest(session_id=session_id, returned_events=event_types)
        if task_filter:
            request.tasks_filters = (
                cast(rawTaskFilters, task_filter.to_disjunction().to_message()),
            )
        if result_filter:
            request.results_filters = (
                cast(rawResultFilters, result_filter.to_disjunction().to_message()),
            )

        streaming_call = self._client.GetEvents(request)
        for message in streaming_call:
            event_type = message.WhichOneof("update")
            if any(
                [
                    event_handler(
                        session_id,
                        EventTypes.from_string(event_type),
                        self._events_obj_mapping[event_type].from_raw_event(
                            getattr(message, event_type)
                        ),
                    )
                    for event_handler in event_handlers
                ]
            ):
                break

    def wait_for_result_availability(self, result_id: str, session_id: str) -> None:
        """Wait until a result is ready i.e its status updates to COMPLETED.

        Args:
            result_id: The ID of the result.
            session_id: The ID of the session.

        Raises:
            RuntimeError: If the result status is ABORTED.
        """

        def handler(session_id, event_type, event):
            if not isinstance(event, ResultStatusUpdateEvent):
                raise ValueError("Handler should receive event of type 'ResultStatusUpdateEvent'.")
            if event.status == ResultStatus.COMPLETED:
                return False
            elif event.status == ResultStatus.ABORTED:
                raise RuntimeError(f"Result {result.name} with ID {result_id} is aborted.")
            return True

        result = self._results_client.get_result(result_id)
        if result.status == ResultStatus.COMPLETED:
            return
        elif result.status == ResultStatus.ABORTED:
            raise RuntimeError(f"Result {result.name} with ID {result_id} is aborted.")

        self.get_events(
            session_id,
            [EventTypes.RESULT_STATUS_UPDATE],
            [handler],
            result_filter=(ResultFieldFilter.RESULT_ID == result_id),
        )
