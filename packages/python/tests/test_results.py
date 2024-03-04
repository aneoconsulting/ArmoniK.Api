import pytest
import warnings

from .conftest import all_rpc_called, rpc_called, get_client
from armonik.client import ArmoniKResults, ResultFieldFilter
from armonik.common import Result, ResultStatus


class TestArmoniKResults:
    def test_get_result(self):
        results_client: ArmoniKResults = get_client("Results")
        result = results_client.get_result("result-name")

        assert rpc_called("Results", "GetResult")
        assert isinstance(result, Result)
        assert result.session_id == "session-id"
        assert result.name == "result-name"
        assert result.owner_task_id == "owner-task-id"
        assert result.status == 2
        assert result.created_at is None
        assert result.completed_at is None
        assert result.result_id == "result-id"
        assert result.size == 0

    def test_get_owner_task_id(self):
        results_client: ArmoniKResults = get_client("Results")
        results_tasks = results_client.get_owner_task_id(["result-id"], "session-id")

        assert rpc_called("Results", "GetOwnerTaskId")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert results_tasks == {}

    def test_list_results_no_filter(self):
        results_client: ArmoniKResults = get_client("Results")
        num, results = results_client.list_results()

        assert rpc_called("Results", "ListResults")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert results == []

    def test_list_results_with_filter(self):
        results_client: ArmoniKResults = get_client("Results")
        num, results = results_client.list_results(
            ResultFieldFilter.STATUS == ResultStatus.COMPLETED
        )

        assert rpc_called("Results", "ListResults", 2)
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert num == 0
        assert results == []

    def test_create_results_metadata(self):
        results_client: ArmoniKResults = get_client("Results")
        results = results_client.create_results_metadata(["result-name"], "session-id")

        assert rpc_called("Results", "CreateResultsMetaData")
        # TODO: Mock must be updated to return something and so that changes the following assertions
        assert results == {}

    def test_create_results(self):
        results_client: ArmoniKResults = get_client("Results")
        results = results_client.create_results({"result-name": b"test data"}, "session-id")

        assert rpc_called("Results", "CreateResults")
        assert results == {}

    def test_get_service_config(self):
        results_client: ArmoniKResults = get_client("Results")
        chunk_size = results_client.get_service_config()

        assert rpc_called("Results", "GetServiceConfiguration")
        assert isinstance(chunk_size, int)
        assert chunk_size == 81920

    def test_upload_result_data(self):
        results_client: ArmoniKResults = get_client("Results")
        result = results_client.upload_result_data("result-name", "session-id", b"test data")

        assert rpc_called("Results", "UploadResultData")
        assert result is None

    def test_download_result_data(self):
        results_client: ArmoniKResults = get_client("Results")
        data = results_client.download_result_data("result-name", "session-id")

        assert rpc_called("Results", "DownloadResultData")
        assert data == b""

    def test_delete_result_data(self):
        results_client: ArmoniKResults = get_client("Results")
        result = results_client.delete_result_data(["result-name"], "session-id")

        assert rpc_called("Results", "DeleteResultsData")
        assert result is None

    def test_watch_results(self):
        results_client: ArmoniKResults = get_client("Results")
        with pytest.raises(NotImplementedError, match=""):
            results_client.watch_results()
        assert rpc_called("Results", "WatchResults", 0)

    def test_get_results_ids(self):
        with warnings.catch_warnings(record=True) as w:
            # Cause all warnings to always be triggered.
            warnings.simplefilter("always")

            results_client: ArmoniKResults = get_client("Results")
            results = results_client.get_results_ids("session-id", ["result_1"])

            assert issubclass(w[-1].category, DeprecationWarning)
            assert rpc_called("Results", "CreateResultsMetaData", 2)
            assert results == {}

    def test_service_fully_implemented(self):
        assert all_rpc_called("Results", missings=["WatchResults"])
