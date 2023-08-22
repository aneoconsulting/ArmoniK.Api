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

using System;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Submitter;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class Submitter : gRPC.V1.Submitter.Submitter.SubmitterBase
{
  /// <inheritdoc />
  [Count]
  public override Task<Configuration> GetServiceConfiguration(Empty             request,
                                                              ServerCallContext context)
    => Task.FromResult(new Configuration
                       {
                         DataChunkMaxSize = 80 * 1024,
                       });

  /// <inheritdoc />
  [Count]
  public override Task<Empty> CancelSession(Session           request,
                                            ServerCallContext context)
    => Task.FromResult(new Empty());

  /// <inheritdoc />
  [Count]
  public override Task<Empty> CancelTasks(TaskFilter        request,
                                          ServerCallContext context)
    => Task.FromResult(new Empty());

  /// <inheritdoc />
  [Count]
  public override Task<CreateSessionReply> CreateSession(CreateSessionRequest request,
                                                         ServerCallContext    context)
    => Task.FromResult(new CreateSessionReply
                       {
                         SessionId = "session-id",
                       });

  /// <inheritdoc />
  [Count]
  public override Task<CreateTaskReply> CreateSmallTasks(CreateSmallTaskRequest request,
                                                         ServerCallContext      context)
    => Task.FromResult(new CreateTaskReply
                       {
                         CreationStatusList = new CreateTaskReply.Types.CreationStatusList(),
                         Error              = "",
                       });


  /// <inheritdoc />
  [Count]
  public override async Task<CreateTaskReply> CreateLargeTasks(IAsyncStreamReader<CreateLargeTaskRequest> requestStream,
                                                               ServerCallContext                          context)
  {
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
  [Count]
  public override Task<Count> CountTasks(TaskFilter        request,
                                         ServerCallContext context)
    => Task.FromResult(new Count());

  /// <inheritdoc />
  [Count]
  public override async Task TryGetResultStream(ResultRequest                    request,
                                                IServerStreamWriter<ResultReply> responseStream,
                                                ServerCallContext                context)
    => await responseStream.WriteAsync(new ResultReply
                                       {
                                         Result = new DataChunk
                                                  {
                                                    DataComplete = true,
                                                  },
                                       })
                           .ConfigureAwait(false);

  /// <inheritdoc />
  [Count]
  public override Task<Count> WaitForCompletion(WaitRequest       request,
                                                ServerCallContext context)
    => Task.FromResult(new Count());

  /// <inheritdoc />
  [Count]
  public override Task<Output> TryGetTaskOutput(TaskOutputRequest request,
                                                ServerCallContext context)
    => Task.FromResult(new Output
                       {
                         Ok = new Empty(),
                       });

  /// <inheritdoc />
  [Obsolete]
  [Count]
  public override Task<AvailabilityReply> WaitForAvailability(ResultRequest     request,
                                                              ServerCallContext context)
    => Task.FromResult(new AvailabilityReply
                       {
                         Ok = new Empty(),
                       });

  /// <inheritdoc />
  [Count]
  public override Task<GetTaskStatusReply> GetTaskStatus(GetTaskStatusRequest request,
                                                         ServerCallContext    context)
    => Task.FromResult(new GetTaskStatusReply());

  /// <inheritdoc />
  [Obsolete]
  [Count]
  public override Task<GetResultStatusReply> GetResultStatus(GetResultStatusRequest request,
                                                             ServerCallContext      context)
    => Task.FromResult(new GetResultStatusReply());

  /// <inheritdoc />
  [Count]
  public override Task<TaskIdList> ListTasks(TaskFilter        request,
                                             ServerCallContext context)
    => Task.FromResult(new TaskIdList());

  /// <inheritdoc />
  [Count]
  public override Task<SessionIdList> ListSessions(SessionFilter     request,
                                                   ServerCallContext context)
    => Task.FromResult(new SessionIdList());

  /// <inheritdoc />
  [Count]
  public override async Task WatchResults(IAsyncStreamReader<WatchResultRequest> requestStream,
                                          IServerStreamWriter<WatchResultStream> responseStream,
                                          ServerCallContext                      context)
  {
    await foreach (var _ in requestStream.ReadAllAsync())
    {
      await responseStream.WriteAsync(new WatchResultStream
                                      {
                                        Status = ResultStatus.Unspecified,
                                      })
                          .ConfigureAwait(false);
    }
  }
}
