from typing import List, Union
import json


class Payload:
    def __init__(self, values: List[Union[float, str]], subtask_threshold=2):
        self.values = values
        self.subtask_threshold = subtask_threshold

    def serialize(self) -> bytes:
        return json.dumps({"values": self.values, "subtask_threshold": self.subtask_threshold}).encode("utf-8")

    @classmethod
    def deserialize(cls, payload: bytes) -> "Payload":
        return cls(**json.loads(payload.decode("utf-8")))


class Result:
    def __init__(self, value: float):
        self.value = value

    def serialize(self) -> bytes:
        return json.dumps({"value": self.value}).encode("utf-8")

    @classmethod
    def deserialize(cls, payload: bytes) -> "Result":
        return cls(**json.loads(payload.decode("utf-8")))
