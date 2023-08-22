// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2023.All rights reserved.
// 
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Tasks;

using Google.Protobuf.WellKnownTypes;

using Grpc.Core;

using TaskStatus = ArmoniK.Api.gRPC.V1.TaskStatus;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class Tasks : gRPC.V1.Tasks.Tasks.TasksBase
{
  private static readonly TaskDetailed MockTask = new()
                                                  {
                                                    Id        = "task-id",
                                                    SessionId = "session-id",
                                                    Status    = TaskStatus.Completed,
                                                    Options = new TaskOptions
                                                              {
                                                                Priority             = 1,
                                                                ApplicationName      = "application-name",
                                                                ApplicationNamespace = "application-namespace",
                                                                ApplicationService   = "application-service",
                                                                ApplicationVersion   = "application-version",
                                                                EngineType           = "engine-type",
                                                                MaxDuration = new Duration
                                                                              {
                                                                                Seconds = 1,
                                                                              },
                                                                MaxRetries  = 1,
                                                                PartitionId = "partition-id",
                                                              },
                                                  };

  /// <inheritdocs />
  [Count]
  public override Task<GetTaskResponse> GetTask(GetTaskRequest    request,
                                                ServerCallContext context)
    => Task.FromResult(new GetTaskResponse
                       {
                         Task = MockTask,
                       });

  /// <inheritdocs />
  [Count]
  public override Task<ListTasksResponse> ListTasks(ListTasksRequest  request,
                                                    ServerCallContext context)
    => Task.FromResult(new ListTasksResponse
                       {
                         Page     = 0,
                         Total    = 0,
                         PageSize = request.PageSize,
                       });

  /// <inheritdocs />
  [Count]
  public override Task<GetResultIdsResponse> GetResultIds(GetResultIdsRequest request,
                                                          ServerCallContext   context)
    => Task.FromResult(new GetResultIdsResponse());

  /// <inheritdocs />
  [Count]
  public override Task<CancelTasksResponse> CancelTasks(CancelTasksRequest request,
                                                        ServerCallContext  context)
    => Task.FromResult(new CancelTasksResponse());

  /// <inheritdocs />
  [Count]
  public override Task<CountTasksByStatusResponse> CountTasksByStatus(CountTasksByStatusRequest request,
                                                                      ServerCallContext         context)
    => Task.FromResult(new CountTasksByStatusResponse());

  /// <inheritdocs />
  [Count]
  public override Task<ListTasksDetailedResponse> ListTasksDetailed(ListTasksRequest  request,
                                                                    ServerCallContext context)
    => Task.FromResult(new ListTasksDetailedResponse
                       {
                         Page     = 0,
                         Total    = 0,
                         PageSize = request.PageSize,
                       });

  /// <inheritdocs />
  [Count]
  public override Task<SubmitTasksResponse> SubmitTasks(SubmitTasksRequest request,
                                                        ServerCallContext  context)
    => Task.FromResult(new SubmitTasksResponse());
}
