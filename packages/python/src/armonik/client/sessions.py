from __future__ import annotations

from typing import List, Optional, Tuple, cast

from deprecation import deprecated
from grpc import Channel

from .. import __version__
from ..common import Direction, Session, TaskOptions
from ..common.filter import Filter, StringFilter, SessionFilter
from ..protogen.client.sessions_service_pb2_grpc import SessionsStub
from ..protogen.common.sessions_common_pb2 import (
    CancelSessionRequest,
    CancelSessionResponse,
    CreateSessionRequest,
    DeleteSessionRequest,
    DeleteSessionResponse,
    GetSessionRequest,
    GetSessionResponse,
    PauseSessionRequest,
    PauseSessionResponse,
    PurgeSessionRequest,
    PurgeSessionResponse,
    CloseSessionRequest,
    CloseSessionResponse,
    ResumeSessionRequest,
    ResumeSessionResponse,
    StopSubmissionRequest,
    StopSubmissionResponse,
    ListSessionsRequest,
    ListSessionsResponse,
)
from ..protogen.common.sessions_fields_pb2 import (
    SessionField,
)
from ..protogen.common.sort_direction_pb2 import SortDirection


@deprecated("3.19.0", None, __version__, "Use Session.<name of the field> instead")
class SessionFieldFilter:
    """
    Enumeration of the available filters
    """

    STATUS = Session.status

    @staticmethod
    def task_options_key(option_key: str) -> StringFilter:
        """
        Filter for the TaskOptions.Options dictionary
        Args:
            option_key: key in the dictionary

        Returns:
            Corresponding filter
        """
        return Session.options[option_key]


class ArmoniKSessions:
    def __init__(self, grpc_channel: Channel):
        """Session service client

        Args:
            grpc_channel: gRPC channel to use
        """
        self._client = SessionsStub(grpc_channel)

    def create_session(
        self,
        default_task_options: TaskOptions,
        partition_ids: Optional[List[str]] = None,
    ) -> str:
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
            partition_ids=partition_ids if partition_ids else [],
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

    def list_sessions(
        self,
        session_filter: Optional[Filter] = None,
        page: int = 0,
        page_size: int = 1000,
        sort_field: Filter = Session.status,
        sort_direction: SortDirection = Direction.ASC,
    ) -> Tuple[int, List[Session]]:
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
            filters=(
                SessionFilter().to_message()
                if session_filter is None
                else session_filter.to_message()
            ),
            sort=ListSessionsRequest.Sort(
                field=cast(SessionField, sort_field.field), direction=sort_direction
            ),
        )
        response: ListSessionsResponse = self._client.ListSessions(request)
        return response.total, [Session.from_message(s) for s in response.sessions]

    def cancel_session(self, session_id: str) -> Session:
        """Cancel a session

        Args:
            session_id: Id of the session to be cancelled
        """
        request = CancelSessionRequest(session_id=session_id)
        response: CancelSessionResponse = self._client.CancelSession(request)
        return Session.from_message(response.session)

    def pause_session(self, session_id: str) -> Session:
        """Pause a session by its id.

        Args:
            session_id: Id of the session to be paused.

        Returns:
            session metadata
        """
        request = PauseSessionRequest(session_id=session_id)
        response: PauseSessionResponse = self._client.PauseSession(request)
        return Session.from_message(response.session)

    def resume_session(self, session_id: str) -> Session:
        """Resume a session by its id.

        Args:
            session_id: Id of the session to be resumed.

        Returns:
            session metadata
        """
        request = ResumeSessionRequest(session_id=session_id)
        response: ResumeSessionResponse = self._client.ResumeSession(request)
        return Session.from_message(response.session)

    def close_session(self, session_id: str) -> Session:
        """Close a session by its id.

        Args:
            session_id: Id of the session to be closed.

        Returns:
            session metadata
        """
        request = CloseSessionRequest(session_id=session_id)
        response: CloseSessionResponse = self._client.CloseSession(request)
        return Session.from_message(response.session)

    def purge_session(self, session_id: str) -> Session:
        """Purge a session by its id.

        Args:
            session_id: Id of the session to be purged.

        Returns:
            session metadata
        """
        request = PurgeSessionRequest(session_id=session_id)
        response: PurgeSessionResponse = self._client.PurgeSession(request)
        return Session.from_message(response.session)

    def delete_session(self, session_id: str) -> Session:
        """Delete a session by its id.

        Args:
            session_id: Id of the session to be deleted.

        Returns:
            session metadata
        """
        request = DeleteSessionRequest(session_id=session_id)
        response: DeleteSessionResponse = self._client.DeleteSession(request)
        return Session.from_message(response.session)

    def stop_submission_session(self, session_id: str, client: bool, worker: bool) -> Session:
        """Stops clients and/or workers from submitting new tasks in the given session.

        Args:
            session_id: Id of the session.
            client: Stops clients from submitting new tasks in the given session.
            worker: Stops workers from submitting new tasks in the given session.

        Returns:
            session metadata
        """
        request = StopSubmissionRequest(session_id=session_id, client=client, worker=worker)
        response: StopSubmissionResponse = self._client.StopSubmission(request)
        return Session.from_message(response.session)
