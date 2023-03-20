from typing import List, Union
import json


class Payload:
    def __init__(self, values: List[Union[float, str]], subtask_threshold=2):
        """
        Creates a payload with a value list and a threshold
        :param values: Values to compute if it's a list of float. If it's a list of strings, corresponds to the results' keys to aggregate
        :param subtask_threshold: Maximum threshold at which the task is split. If number of values is less than this threshold, the task is computed
        """
        self.values = values
        self.subtask_threshold = subtask_threshold

    def serialize(self) -> bytes:
        """
        Serializes the payload. Converts the attributes to json and return a byte array
        :return: Serialized payload compatible with ArmoniK
        """
        return json.dumps({"values": self.values, "subtask_threshold": self.subtask_threshold}).encode("utf-8")

    @classmethod
    def deserialize(cls, payload: bytes) -> "Payload":
        """
        Create a payload instance from the payload bytes received from ArmoniK
        :param payload: Raw ArmoniK Payload
        :return: Payload object
        """
        return cls(**json.loads(payload.decode("utf-8")))


class Result:
    def __init__(self, value: float):
        """
        Result of a task
        :param value: Actual value
        """
        self.value = value

    def serialize(self) -> bytes:
        """
        Serializes the result. Converts the attributes to json and return a byte array
        :return: Serialized result compatible with ArmoniK
        """
        return json.dumps({"value": self.value}).encode("utf-8")

    @classmethod
    def deserialize(cls, payload: bytes) -> "Result":
        """
        Create a Result instance from the data dependency bytes received from ArmoniK
        :param payload: Raw ArmoniK data dependency
        :return: Result object
        """
        return cls(**json.loads(payload.decode("utf-8")))
