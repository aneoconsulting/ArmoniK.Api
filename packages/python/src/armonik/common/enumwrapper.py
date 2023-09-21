from __future__ import annotations

from ..protogen.common.task_status_pb2 import *
from ..protogen.common.worker_common_pb2 import HealthCheckReply
from ..protogen.common.sort_direction_pb2 import SORT_DIRECTION_ASC, SORT_DIRECTION_DESC

# This file is necessary because the grpc types aren't considered proper types


class HealthCheckStatus:
    NOT_SERVING = HealthCheckReply.NOT_SERVING
    SERVING = HealthCheckReply.SERVING
    UNKNOWN = HealthCheckReply.UNKNOWN


class TaskStatus:
    CANCELLED = TASK_STATUS_CANCELLED
    CANCELLING = TASK_STATUS_CANCELLING
    COMPLETED = TASK_STATUS_COMPLETED
    CREATING = TASK_STATUS_CREATING
    DISPATCHED = TASK_STATUS_DISPATCHED
    ERROR = TASK_STATUS_ERROR
    PROCESSED = TASK_STATUS_PROCESSED
    PROCESSING = TASK_STATUS_PROCESSING
    SUBMITTED = TASK_STATUS_SUBMITTED
    TIMEOUT = TASK_STATUS_TIMEOUT
    UNSPECIFIED = TASK_STATUS_UNSPECIFIED


class Direction:
    ASC = SORT_DIRECTION_ASC
    DESC = SORT_DIRECTION_DESC
