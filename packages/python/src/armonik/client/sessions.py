from __future__ import annotations
from grpc import Channel
from typing import cast, Tuple, List, Optional

from ..protogen.client.sessions_service_pb2_grpc import SessionsStub
from ..protogen.common.submitter_common_pb2 import SessionFilter
from ..protogen.common.sessions_common_pb2 import GetSessionRequest, GetSessionResponse, ListSessionsRequest, ListSessionsResponse, SessionRaw, CancelSessionRequest, CancelSessionResponse, CreateSessionRequest
from ..protogen.common.sessions_filters_pb2 import Filters as rawFilters, FiltersAnd as rawFilterAnd, FilterField as rawFilterField, FilterStatus as rawFilterStatus
from ..protogen.common.sessions_fields_pb2 import *
from ..common.filter import StringFilter, StatusFilter, DateFilter, NumberFilter, Filter
from ..protogen.common.sort_direction_pb2 import SortDirection
from ..common import Direction, Session, TaskOptions
from ..protogen.common.sessions_fields_pb2 import SessionField, SessionRawField, SESSION_RAW_ENUM_FIELD_STATUS, TaskOptionGenericField

class SessionFieldFilter:
    """
    Enumeration of the available filters
    """
    STATUS = StatusFilter(SessionField(session_raw_field=SessionRawField(field=SESSION_RAW_ENUM_FIELD_STATUS)), rawFilters, rawFilterAnd, rawFilterField, rawFilterStatus)

    @staticmethod
    def task_options_key(option_key: str) -> StringFilter:
        """
        Filter for the TaskOptions.Options dictionary
        Args:
            option_key: key in the dictionary

        Returns:
            Corresponding filter
        """
        return StringFilter(SessionField(task_option_generic_field=TaskOptionGenericField(field=option_key)), rawFilters, rawFilterAnd, rawFilterField)

class ArmoniKSessions:

    def __init__(self, grpc_channel: Channel):
        """ Session service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = SessionsStub(grpc_channel)

    def create_session(self, default_task_options: TaskOptions, partition_ids: Optional[List[str]] = None) -> str:
        """Create a session

        Args:
            default_task_options: Default TaskOptions used when
                submitting tasks without specifying the options
            partition_ids: List of partitions this session can send
                tasks to. If unspecified, can only send to the default
                partition

        Returns:
            Session Id
        """
        request = CreateSessionRequest(
            default_task_option=default_task_options.to_message(),
            partition_ids=partition_ids if partition_ids else []
        )
        return self._client.CreateSession(request).session_id

    def get_session(self, session_id: str):
        """Get a session by its ID.
        
        Args:
            session_id: The ID of the session.
        
        Return:
            The session summary.
        """
        request = GetSessionRequest(session_id=session_id)
        response: GetSessionResponse = self._client.GetSession(request)
        return Session.from_message(response.session)

    def list_sessions(self, session_filter: Filter | None = None, page: int = 0, page_size: int = 1000, sort_field: Filter = SessionFieldFilter.STATUS, sort_direction: SortDirection = Direction.ASC) -> Tuple[int, List[Session]]:
        """
        List sessions

        Args:
            session_filter (Filter): Filter to apply when listing sessions.
            page: page number to request, this can be useful when paginating the result, defaults to 0
            page_size: size of a page, defaults to 1000
            sort_field: field on which to sort the resulting list, defaults to the status
            sort_direction: direction of the sort, defaults to ascending

        Returns:
            A tuple containing :
            - The total number of sessions for the given filter
            - The obtained list of sessions
        """
        request = ListSessionsRequest(
            page=page,
            page_size=page_size,
            filters=cast(rawFilters, session_filter.to_disjunction().to_message()) if session_filter else None,
            sort=ListSessionsRequest.Sort(field=cast(SessionField, sort_field.field), direction=sort_direction),
        )
        response : ListSessionsResponse = self._client.ListSessions(request)
        return response.total, [Session.from_message(s) for s in response.sessions]

    def cancel_session(self, session_id: str) -> None:
        """Cancel a session

        Args:
            session_id: Id of the session to b cancelled
        """
        self._client.CancelSession(CancelSessionRequest(session_id=session_id))
