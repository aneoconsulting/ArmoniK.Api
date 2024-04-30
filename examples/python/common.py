from typing import List



class TaskIO:
    def __init__(self, values: List[int]):
        """
        Initialize TaskIO class with a list of float values
        """
        self.values = values

    def serialize(self) -> bytes:
        """
        Method to serialize the object into bytes
        """
        return bytes(self.values)

    @classmethod
    def deserialize(cls, data: bytes) -> "TaskIO":
        """
        Method to deserialize bytes and reconstruct a TaskIO object
        """
        values = list(data)
        return cls(values)
