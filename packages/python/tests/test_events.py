from .conftest import all_rpc_called, rpc_called, get_client
from armonik.client import ArmoniKEvents
from armonik.common import EventTypes, NewResultEvent, ResultStatus


class TestArmoniKEvents:
    def test_get_events_no_filter(self):
        def test_handler(session_id, event_type, event):
            assert session_id == "session-id"
            assert event_type == EventTypes.NEW_RESULT
            assert isinstance(event, NewResultEvent)
            assert event.result_id == "result-id"
            assert event.owner_id == "owner-id"
            assert event.status == ResultStatus.CREATED

        tasks_client: ArmoniKEvents = get_client("Events")
        tasks_client.get_events("session-id", [EventTypes.TASK_STATUS_UPDATE], [test_handler])

        assert rpc_called("Events", "GetEvents")

    def test_service_fully_implemented(self):
        assert all_rpc_called("Events")
