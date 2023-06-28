// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2023.All rights reserved.
//   W.Kirschenmann   <wkirschenmann@aneo.fr>
//   J.Gurhem         <jgurhem@aneo.fr>
//   D.Dubuc          <ddubuc@aneo.fr>
//   L.Ziane Khodja   <lzianekhodja@aneo.fr>
//   F.Lemaitre       <flemaitre@aneo.fr>
//   S.Djebbar        <sdjebbar@aneo.fr>
//   J.Fonseca        <jfonseca@aneo.fr>
//
// This program is free software:you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY, without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.If not, see <http://www.gnu.org/licenses/>.

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Tasks;

using Google.Protobuf.WellKnownTypes;

using Grpc.Core;

using TaskStatus = ArmoniK.Api.gRPC.V1.TaskStatus;

namespace ArmoniK.Api.Mock.Services;

public class TasksService : Tasks.TasksBase
{
  private static readonly TaskRaw MockTask = new()
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

  public CallCount Calls = new();


  /// <inheritdocs />
  public override Task<GetTaskResponse> GetTask(GetTaskRequest    request,
                                                ServerCallContext context)
  {
    Interlocked.Add(ref Calls.GetTask,
                    1);
    return Task.FromResult(new GetTaskResponse
                           {
                             Task = MockTask,
                           });
  }

  /// <inheritdocs />
  public override Task<ListTasksResponse> ListTasks(ListTasksRequest  request,
                                                    ServerCallContext context)
  {
    Interlocked.Add(ref Calls.ListTasks,
                    1);
    return Task.FromResult(new ListTasksResponse
                           {
                             Page     = 0,
                             Total    = 0,
                             PageSize = request.PageSize,
                           });
  }

  /// <inheritdocs />
  public override Task<GetResultIdsResponse> GetResultIds(GetResultIdsRequest request,
                                                          ServerCallContext   context)
  {
    Interlocked.Add(ref Calls.GetResultIds,
                    1);
    return Task.FromResult(new GetResultIdsResponse());
  }

  /// <inheritdocs />
  public override Task<CancelTasksResponse> CancelTasks(CancelTasksRequest request,
                                                        ServerCallContext  context)
  {
    Interlocked.Add(ref Calls.CancelTasks,
                    1);
    return Task.FromResult(new CancelTasksResponse());
  }

  /// <inheritdocs />
  public override Task<CountTasksByStatusResponse> CountTasksByStatus(CountTasksByStatusRequest request,
                                                                      ServerCallContext         context)
  {
    Interlocked.Add(ref Calls.CountTasksByStatus,
                    1);
    return Task.FromResult(new CountTasksByStatusResponse());
  }

  /// <inheritdocs />
  public override Task<ListTasksRawResponse> ListTasksRaw(ListTasksRequest  request,
                                                          ServerCallContext context)
  {
    Interlocked.Add(ref Calls.ListTasksRaw,
                    1);
    return Task.FromResult(new ListTasksRawResponse
                           {
                             Page     = 0,
                             Total    = 0,
                             PageSize = request.PageSize,
                           });
  }

  /// <inheritdocs />
  public override Task<SubmitTasksResponse> SubmitTasks(SubmitTasksRequest request,
                                                        ServerCallContext  context)
  {
    Interlocked.Add(ref Calls.SubmitTasks,
                    1);
    return Task.FromResult(new SubmitTasksResponse());
  }

  public struct CallCount
  {
    public int GetTask            = 0;
    public int ListTasks          = 0;
    public int GetResultIds       = 0;
    public int CancelTasks        = 0;
    public int CountTasksByStatus = 0;
    public int ListTasksRaw       = 0;
    public int SubmitTasks        = 0;

    public CallCount()
    {
    }
  }
}
