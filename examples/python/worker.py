from armonik.worker import ArmoniKWorker, TaskHandler
from armonik.common import Output


# Actual computation
def processor(task_handler: TaskHandler) -> Output:
    return Output()


def main():

