from dataclasses import dataclass, field
from argparse import ArgumentParser
from typing import List, Union
from .rpcs import *
import subprocess

@dataclass
class Feature:
    """Class that holds an ArmoniK feature
    """
    # Feature short description
    description: str
    # Main procedure to use
    main_procedure: Union[RPC, List[RPC]]
    # List of alternative procedures
    alternatives: List[Union[RPC, List[RPC]]] = field(default_factory=list)


########### TO BE REGULARLY UPDATED #############

########### CLIENT SIDE ############
client_features: List[Feature] = [
    Feature("Create task", [Tasks.SubmitTasks, Results.CreateResultsMetaData, Results.UploadResultData], [[Tasks.SubmitTasks, Results.CreateResults],[Submitter.CreateLargeTasks, Results.CreateResultsMetaData], [Submitter.CreateSmallTasks, Results.CreateResultsMetaData]]),
    Feature("Create session", Sessions.CreateSession, [Submitter.CreateSession]),
    Feature("Get service configuration", Results.GetServiceConfiguration, [Submitter.GetServiceConfiguration]),
    Feature("Create new results", Results.CreateResultsMetaData),
    Feature("Get task associated with result", Results.GetOwnerTaskId),
    Feature("Create a new result with data", Results.CreateResults, [[Results.CreateResultsMetaData, Results.UploadResultData]]),
    Feature("List applications", Applications.ListApplications),
    Feature("Get user information", Auth.GetCurrentUser),
    Feature("Monitor events", Events.GetEvents),
    Feature("Get partition properties", Partitions.GetPartition),
    Feature("List all partition", Partitions.ListPartitions),
    Feature("List results", Results.ListResults),
    Feature("Delete result", Results.DeleteResultsData),
    Feature("Download result", Results.DownloadResultData, [Submitter.TryGetResultStream]),
    Feature("Upload result data", Results.UploadResultData),
    Feature("Get result properties", Results.GetResult),
    Feature("Cancel session and its tasks", Sessions.CancelSession, [Submitter.CancelSession]),
    Feature("Get session properties", Sessions.GetSession),
    Feature("List sessions", Sessions.ListSessions, [Submitter.ListSessions]),
    Feature("Cancel tasks", Tasks.CancelTasks, [Submitter.CancelTasks]),
    Feature("Count tasks", Submitter.CountTasks),
    Feature("Get task output", Submitter.TryGetTaskOutput),
    Feature("Wait until the result is available", Submitter.WaitForAvailability),
    Feature("Wait for task completion", Submitter.WaitForCompletion),
    Feature("List tasks", Tasks.ListTasks, [Tasks.ListTasksDetailed, Submitter.ListTasks]),
    Feature("Get tasks statuses", Tasks.ListTasks, [Tasks.ListTasksDetailed, Tasks.GetTask, Submitter.GetTaskStatus]),
    Feature("Get results statuses", Results.ListResults, [Results.GetResult, Submitter.GetResultStatus]),
    Feature("Get detailed task properties", Tasks.GetTask, [Tasks.ListTasksDetailed]),
    Feature("Get task result ids", Tasks.GetResultIds, [Tasks.ListTasksDetailed]),
    Feature("Count task by status", Tasks.CountTasksByStatus),
    Feature("Get core versions", Versions.ListVersions)
]

########### WORKER SIDE ############
worker_features: List[Feature] = [
    Feature("Send result", Agent.UploadResultData, [Agent.SendResult]),
    Feature("Create task without creating new results", Agent.CreateTask),
    Feature("Create task with new results", [Agent.CreateTask, Agent.CreateResultsMetaData]),
    Feature("Create new results", Agent.CreateResultsMetaData),
    Feature("Create a new result with data", Agent.CreateResults, [[Agent.CreateResultsMetaData, Agent.UploadResultData], [Agent.CreateResultsMetaData, Agent.SendResult]]),
    Feature("Send multiple results", Agent.SendResult),
    Feature("Get resource data", Agent.GetResourceData),
]

########### UNIMPLEMENTED ############
# These are the features that currently have no implementation in Core and thus cannot be used
unimplemented_features : List[Feature] = [
    Feature("Get common data", Agent.GetCommonData),
    Feature("Get direct data", Agent.GetDirectData),
    Feature("Watch for results", Submitter.WatchResults)
]

# If an endpoint exists and is not in thoses lists, it will be considered unknown and will trigger a warning

#################################################

class OutputType:
    CONSOLE=0
    HTML=1


def main(excluded:List[str], calls_json:str, output:int=OutputType.CONSOLE, output_file:str="feature_coverage.html"):
    # Checks the covered features against the given lists
    pass


if __name__ == "__main__":
    argparser = ArgumentParser("Check Api Coverage", description="Checks that the implementation")
    argparser.add_argument("-a", "--address", type=str, required=True)
    argparser.add_argument("-r", "--report", type=str, required=False)
    argparser.add_argument("-x", "--exclude", nargs='+', default=[])
    argparser.add_argument("--exclude_file", type=str)
    args = argparser.parse_args()
    with open(args.exclude_file, "r") as f:
        args.exclude.extend([l[:-1] for l in f.readlines()])
    main(args.exclude, str(subprocess.run(["curl", args.address], check=True, capture_output=True).stdout), OutputType.HTML if args.report is not None else OutputType.CONSOLE, args.report if args.report is not None else "feature_coverage.html")