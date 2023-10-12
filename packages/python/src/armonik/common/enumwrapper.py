from __future__ import annotations

from ..protogen.common.task_status_pb2 import TaskStatus as RawStatus, _TASKSTATUS, TASK_STATUS_CANCELLED, TASK_STATUS_CANCELLING, TASK_STATUS_COMPLETED, TASK_STATUS_CREATING, TASK_STATUS_DISPATCHED, TASK_STATUS_ERROR, TASK_STATUS_PROCESSED, TASK_STATUS_PROCESSING, TASK_STATUS_SUBMITTED, TASK_STATUS_TIMEOUT, TASK_STATUS_UNSPECIFIED, TASK_STATUS_RETRIED
from ..protogen.common.session_status_pb2 import SessionStatus as RawSessionStatus, _SESSIONSTATUS, SESSION_STATUS_UNSPECIFIED, SESSION_STATUS_CANCELLED, SESSION_STATUS_RUNNING
from ..protogen.common.worker_common_pb2 import HealthCheckReply
from ..protogen.common.sort_direction_pb2 import SORT_DIRECTION_ASC, SORT_DIRECTION_DESC

# This file is necessary because the grpc types aren't considered proper types


class HealthCheckStatus:
    NOT_SERVING = HealthCheckReply.NOT_SERVING
    SERVING = HealthCheckReply.SERVING
    UNKNOWN = HealthCheckReply.UNKNOWN


class TaskStatus:
    @staticmethod
    def name_from_value(status: RawStatus) -> str:
        return _TASKSTATUS.values_by_number[status].name

    CANCELLED = TASK_STATUS_CANCELLED
    CANCELLING = TASK_STATUS_CANCELLING
    COMPLETED = TASK_STATUS_COMPLETED
    CREATING = TASK_STATUS_CREATING
    DISPATCHED = TASK_STATUS_DISPATCHED
    ERROR = TASK_STATUS_ERROR
    PROCESSED = TASK_STATUS_PROCESSED
    PROCESSING = TASK_STATUS_PROCESSING
    SUBMITTED = TASK_STATUS_SUBMITTED
    RETRIED = TASK_STATUS_RETRIED
    TIMEOUT = TASK_STATUS_TIMEOUT
    UNSPECIFIED = TASK_STATUS_UNSPECIFIED


class Direction:
    ASC = SORT_DIRECTION_ASC
    DESC = SORT_DIRECTION_DESC


class SessionStatus:
    @staticmethod
    def name_from_value(status: RawSessionStatus) -> str:
        return _SESSIONSTATUS.values_by_number[status].name

    UNSPECIFIED = SESSION_STATUS_UNSPECIFIED
    RUNNING = SESSION_STATUS_RUNNING
    CANCELLED = SESSION_STATUS_CANCELLED
    