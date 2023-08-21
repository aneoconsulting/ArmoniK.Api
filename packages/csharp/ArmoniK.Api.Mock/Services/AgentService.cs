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
using ArmoniK.Api.gRPC.V1.Agent;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class AgentService : Agent.AgentBase
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
