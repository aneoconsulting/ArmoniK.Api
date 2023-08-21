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

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class Agent : gRPC.V1.Agent.Agent.AgentBase
{
  /// <inheritdocs />
  [Count]
  public override Task<CreateTaskReply> CreateTask(IAsyncStreamReader<CreateTaskRequest> requestStream,
                                                   ServerCallContext                     context)
    => Task.FromResult(new CreateTaskReply
                       {
                         CommunicationToken = "communication-token",
                         CreationStatusList = new CreateTaskReply.Types.CreationStatusList(),
                       });

  /// <inheritdocs />
  [Count]
  public override async Task GetCommonData(DataRequest                    request,
                                           IServerStreamWriter<DataReply> responseStream,
                                           ServerCallContext              context)
    => await responseStream.WriteAsync(new DataReply
                                       {
                                         Data = new DataChunk
                                                {
                                                  DataComplete = true,
                                                },
                                       })
                           .ConfigureAwait(false);

  /// <inheritdocs />
  [Count]
  public override async Task GetDirectData(DataRequest                    request,
                                           IServerStreamWriter<DataReply> responseStream,
                                           ServerCallContext              context)
    => await responseStream.WriteAsync(new DataReply
                                       {
                                         Data = new DataChunk
                                                {
                                                  DataComplete = true,
                                                },
                                       })
                           .ConfigureAwait(false);

  /// <inheritdocs />
  [Count]
  public override async Task GetResourceData(DataRequest                    request,
                                             IServerStreamWriter<DataReply> responseStream,
                                             ServerCallContext              context)
    => await responseStream.WriteAsync(new DataReply
                                       {
                                         Data = new DataChunk
                                                {
                                                  DataComplete = true,
                                                },
                                       })
                           .ConfigureAwait(false);

  /// <inheritdocs />
  [Count]
  public override async Task<ResultReply> SendResult(IAsyncStreamReader<Result> requestStream,
                                                     ServerCallContext          context)
  {
    await foreach (var _ in requestStream.ReadAllAsync())
    {
    }

    return new ResultReply
           {
             Ok = new Empty(),
           };
  }

  /// <inheritdocs />
  [Count]
  public override Task<CreateResultsMetaDataResponse> CreateResultsMetaData(CreateResultsMetaDataRequest request,
                                                                            ServerCallContext            context)
    => Task.FromResult(new CreateResultsMetaDataResponse
                       {
                         CommunicationToken = request.CommunicationToken,
                       });

  /// <inheritdocs />
  [Count]
  public override Task<SubmitTasksResponse> SubmitTasks(SubmitTasksRequest request,
                                                        ServerCallContext  context)
    => Task.FromResult(new SubmitTasksResponse
                       {
                         CommunicationToken = request.CommunicationToken,
                       });

  /// <inheritdocs />
  [Count]
  public override async Task<UploadResultDataResponse> UploadResultData(IAsyncStreamReader<UploadResultDataRequest> requestStream,
                                                                        ServerCallContext                           context)
  {
    await foreach (var _ in requestStream.ReadAllAsync())
    {
    }

    return new UploadResultDataResponse
           {
             ResultId           = "result-id",
             CommunicationToken = "communication-token",
           };
  }

  /// <inheritdocs />
  [Count]
  public override Task<CreateResultsResponse> CreateResults(CreateResultsRequest request,
                                                            ServerCallContext    context)
    => Task.FromResult(new CreateResultsResponse
                       {
                         CommunicationToken = request.CommunicationToken,
                       });
}
