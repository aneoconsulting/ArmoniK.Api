// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2023. All rights reserved.
//   W. Kirschenmann   <wkirschenmann@aneo.fr>
//   J. Gurhem         <jgurhem@aneo.fr>
//   D. Dubuc          <ddubuc@aneo.fr>
//   L. Ziane Khodja   <lzianekhodja@aneo.fr>
//   F. Lemaitre       <flemaitre@aneo.fr>
//   S. Djebbar        <sdjebbar@aneo.fr>
//   J. Fonseca        <jfonseca@aneo.fr>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY, without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Submitter;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class SubmitterService : Submitter.SubmitterBase
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
    await foreach (var req in requestStream.ReadAllAsync())
    {
      await responseStream.WriteAsync(new WatchResultStream
                                      {
                                        Status = ResultStatus.Unspecified,
                                      })
                          .ConfigureAwait(false);
    }
  }
}
