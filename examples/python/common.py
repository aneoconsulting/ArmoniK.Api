import struct
from typing import List, Union

class InputPayload:
    def __init__(self, values: List[Union[int, str]]):
        """
        Creates an input payload
        Args:
            values: if it's a list of integers, will be the values to compute. If it's a list of strings, keys to aggregate
        """
        self.values: List[Union[int, str]] = values
        self.aggregate:bool = isinstance(self.values[0], str) if self.values else False

    def serialize(self) -> bytes:
        common = struct.pack('>?I', self.aggregate, len(self.values))
        if self.aggregate:
            return common+struct.pack(''.join(["36p"]*len(self.values)), *[v.encode('ascii') for v in self.values])
        return common+(b''.join(v.to_bytes(4, "little",signed=True) for v in self.values))

    @classmethod
    def deserialize(cls, data: bytes) -> "InputPayload":
        aggregate, length = struct.unpack('>?I', data[:5])
        if aggregate:
            return cls([s.decode('ascii') for s in struct.unpack(''.join(["36p"]*length),data[5:])])
        return cls([int.from_bytes(data[i:i+4], "little", signed=True) for i in range(5, len(data), 4)])

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
