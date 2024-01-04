import datetime

from .conftest import all_rpc_called, rpc_called, get_client
from armonik.client import ArmoniKSessions, SessionFieldFilter
from armonik.common import Session, SessionStatus, TaskOptions


class TestArmoniKSessions:

    def test_create_session(self):
        sessions_client: ArmoniKSessions = get_client("Sessions")
        default_task_options = TaskOptions(
            max_duration=datetime.timedelta(seconds=1),
            priority=1,
            max_retries=1
        )
        session_id = sessions_client.create_session(default_task_options)

        assert rpc_called("Sessions", "CreateSession")
        assert session_id == "session-id"

    def test_get_session(self):
        sessions_client: ArmoniKSessions = get_client("Sessions")
        session = sessions_client.get_session("session-id")

        assert rpc_called("Sessions", "GetSession")
        assert isinstance(session, Session)
        assert session.session_id == 'session-id'
        assert session.status == SessionStatus.CANCELLED
        assert session.partition_ids == []
        assert session.options == TaskOptions(
            max_duration=datetime.timedelta(0),
            priority=0,
            max_retries=0,
            partition_id='',
            application_name='',
            application_version='',
            application_namespace='',
            application_service='',
            engine_type='',
            options={}
        )
        assert session.created_at == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)
        assert session.cancelled_at == datetime.datetime(1970, 1, 1, 0, 0, tzinfo=datetime.timezone.utc)
        assert session.duration == datetime.timedelta(0)

    def test_list_session_no_filter(self):
        sessions_client: ArmoniKSessions = get_client("Sessions")
        num, sessions = sessions_client.list_sessions()

        assert rpc_called("Sessions", "ListSessions")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert sessions == []

    def test_list_session_with_filter(self):
        sessions_client: ArmoniKSessions = get_client("Sessions")
        num, sessions = sessions_client.list_sessions(SessionFieldFilter.STATUS == SessionStatus.RUNNING)

        assert rpc_called("Sessions", "ListSessions", 2)
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert sessions == []

    def test_cancel_session(self):
        sessions_client: ArmoniKSessions = get_client("Sessions")
        sessions_client.cancel_session("session-id")

        assert rpc_called("Sessions", "CancelSession")

    def test_service_fully_implemented(self):
        assert all_rpc_called("Sessions")
