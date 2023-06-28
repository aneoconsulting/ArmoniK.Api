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
using ArmoniK.Api.gRPC.V1.Submitter;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

public class SubmitterService : Submitter.SubmitterBase
{
  public CallCount Calls = new();

  /// <inheritdoc />
  public override Task<Configuration> GetServiceConfiguration(Empty             request,
                                                              ServerCallContext context)
  {
    Interlocked.Add(ref Calls.GetServiceConfiguration,
                    1);
    return Task.FromResult(new Configuration
                           {
                             DataChunkMaxSize = 80 * 1024,
                           });
  }

  /// <inheritdoc />
  public override Task<Empty> CancelSession(Session           request,
                                            ServerCallContext context)
  {
    Interlocked.Add(ref Calls.CancelSession,
                    1);
    return Task.FromResult(new Empty());
  }

  /// <inheritdoc />
  public override Task<Empty> CancelTasks(TaskFilter        request,
                                          ServerCallContext context)
  {
    Interlocked.Add(ref Calls.CancelTasks,
                    1);
    return Task.FromResult(new Empty());
  }

  /// <inheritdoc />
  public override Task<CreateSessionReply> CreateSession(CreateSessionRequest request,
                                                         ServerCallContext    context)
  {
    Interlocked.Add(ref Calls.CreateSession,
                    1);
    return Task.FromResult(new CreateSessionReply
                           {
                             SessionId = "session-id",
                           });
  }

  /// <inheritdoc />
  public override Task<CreateTaskReply> CreateSmallTasks(CreateSmallTaskRequest request,
                                                         ServerCallContext      context)
  {
    Interlocked.Add(ref Calls.CreateSmallTasks,
                    1);
    return Task.FromResult(new CreateTaskReply
                           {
                             CreationStatusList = new CreateTaskReply.Types.CreationStatusList(),
                             Error              = "",
                           });
  }


  /// <inheritdoc />
  public override async Task<CreateTaskReply> CreateLargeTasks(IAsyncStreamReader<CreateLargeTaskRequest> requestStream,
                                                               ServerCallContext                          context)
  {
    Interlocked.Add(ref Calls.CreateLargeTasks,
                    1);
    await foreach (var _ in requestStream.ReadAllAsync())
    {
    }

    return new CreateTaskReply
           {
             CreationStatusList = new CreateTaskReply.Types.CreationStatusList(),
             Error              = "",
           };
  }

  /// <inheritdoc />
  public override Task<Count> CountTasks(TaskFilter        request,
                                         ServerCallContext context)
  {
    Interlocked.Add(ref Calls.CountTasks,
                    1);
    return Task.FromResult(new Count());
  }

  /// <inheritdoc />
  public override async Task TryGetResultStream(ResultRequest                    request,
                                                IServerStreamWriter<ResultReply> responseStream,
                                                ServerCallContext                context)
  {
    Interlocked.Add(ref Calls.TryGetResultStream,
                    1);
    await responseStream.WriteAsync(new ResultReply
                                    {
                                      Result = new DataChunk
                                               {
                                                 DataComplete = true,
                                               },
                                    })
                        .ConfigureAwait(false);
  }

  /// <inheritdoc />
  public override Task<Count> WaitForCompletion(WaitRequest       request,
                                                ServerCallContext context)
  {
    Interlocked.Add(ref Calls.WaitForCompletion,
                    1);
    return Task.FromResult(new Count());
  }

  /// <inheritdoc />
  public override Task<Output> TryGetTaskOutput(TaskOutputRequest request,
                                                ServerCallContext context)
  {
    Interlocked.Add(ref Calls.TryGetTaskOutput,
                    1);
    return Task.FromResult(new Output
                           {
                             Ok = new Empty(),
                           });
  }

  /// <inheritdoc />
  [Obsolete]
  public override Task<AvailabilityReply> WaitForAvailability(ResultRequest     request,
                                                              ServerCallContext context)
  {
    Interlocked.Add(ref Calls.WaitForAvailability,
                    1);
    return Task.FromResult(new AvailabilityReply
                           {
                             Ok = new Empty(),
                           });
  }

  /// <inheritdoc />
  public override Task<GetTaskStatusReply> GetTaskStatus(GetTaskStatusRequest request,
                                                         ServerCallContext    context)
  {
    Interlocked.Add(ref Calls.GetTaskStatus,
                    1);
    return Task.FromResult(new GetTaskStatusReply());
  }

  /// <inheritdoc />
  [Obsolete]
  public override Task<GetResultStatusReply> GetResultStatus(GetResultStatusRequest request,
                                                             ServerCallContext      context)
  {
    Interlocked.Add(ref Calls.GetResultStatus,
                    1);
    return Task.FromResult(new GetResultStatusReply());
  }

  /// <inheritdoc />
  public override Task<TaskIdList> ListTasks(TaskFilter        request,
                                             ServerCallContext context)
  {
    Interlocked.Add(ref Calls.ListTasks,
                    1);

    return Task.FromResult(new TaskIdList());
  }

  /// <inheritdoc />
  public override Task<SessionIdList> ListSessions(SessionFilter     request,
                                                   ServerCallContext context)
  {
    Interlocked.Add(ref Calls.ListSessions,
                    1);
    return Task.FromResult(new SessionIdList());
  }

  /// <inheritdoc />
  public override async Task WatchResults(IAsyncStreamReader<WatchResultRequest> requestStream,
                                          IServerStreamWriter<WatchResultStream> responseStream,
                                          ServerCallContext                      context)
  {
    await foreach (var req in requestStream.ReadAllAsync())
    {
      await responseStream.WriteAsync(new WatchResultStream
                                      {
                                        Status = ResultStatus.Unspecified,
                                      })
                          .ConfigureAwait(false);
    }
  }

  public struct CallCount
  {
    public int GetServiceConfiguration = 0;
    public int CancelSession           = 0;
    public int CancelTasks             = 0;
    public int CreateSession           = 0;
    public int CreateSmallTasks        = 0;
    public int CreateLargeTasks        = 0;
    public int CountTasks              = 0;
    public int TryGetResultStream      = 0;
    public int WaitForCompletion       = 0;
    public int TryGetTaskOutput        = 0;
    public int WaitForAvailability     = 0;
    public int GetTaskStatus           = 0;
    public int GetResultStatus         = 0;
    public int ListTasks               = 0;
    public int ListSessions            = 0;

    public CallCount()
    {
    }
  }
}
