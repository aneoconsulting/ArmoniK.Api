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
using ArmoniK.Api.gRPC.V1.Results;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class Results : gRPC.V1.Results.Results.ResultsBase
{
  private static readonly ResultRaw MockResult = new()
                                                 {
                                                   SessionId   = "session-id",
                                                   ResultId    = "result-id",
                                                   Name        = "result-name",
                                                   Status      = ResultStatus.Completed,
                                                   OwnerTaskId = "owner-task-id",
                                                 };

  /// <inheritdocs />
  [Count]
  public override Task<GetOwnerTaskIdResponse> GetOwnerTaskId(GetOwnerTaskIdRequest request,
                                                              ServerCallContext     context)
    => Task.FromResult(new GetOwnerTaskIdResponse
                       {
                         SessionId = "session-id",
                       });

  /// <inheritdocs />
  [Count]
  public override Task<ListResultsResponse> ListResults(ListResultsRequest request,
                                                        ServerCallContext  context)
    => Task.FromResult(new ListResultsResponse
                       {
                         Page     = 0,
                         Total    = 0,
                         PageSize = request.PageSize,
                       });

  /// <inheritdocs />
  [Count]
  public override Task<CreateResultsMetaDataResponse> CreateResultsMetaData(CreateResultsMetaDataRequest request,
                                                                            ServerCallContext            context)
    => Task.FromResult(new CreateResultsMetaDataResponse());

  /// <inheritdocs />
  [Count]
  public override Task<CreateResultsResponse> CreateResults(CreateResultsRequest request,
                                                            ServerCallContext    context)
    => Task.FromResult(new CreateResultsResponse());

  /// <inheritdocs />
  [Count]
  public override Task<DeleteResultsDataResponse> DeleteResultsData(DeleteResultsDataRequest request,
                                                                    ServerCallContext        context)
    => Task.FromResult(new DeleteResultsDataResponse
                       {
                         SessionId = "session-id",
                       });

  /// <inheritdocs />
  [Count]
  public override Task DownloadResultData(DownloadResultDataRequest                       request,
                                          IServerStreamWriter<DownloadResultDataResponse> responseStream,
                                          ServerCallContext                               context)
    => Task.CompletedTask;

  /// <inheritdocs />
  [Count]
  public override Task<ResultsServiceConfigurationResponse> GetServiceConfiguration(Empty             request,
                                                                                    ServerCallContext context)
    => Task.FromResult(new ResultsServiceConfigurationResponse
                       {
                         DataChunkMaxSize = 80 * 1024,
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
             Result = MockResult,
           };
  }

  /// <inheritdocs />
  [Count]
  public override Task<GetResultResponse> GetResult(GetResultRequest  request,
                                                    ServerCallContext context)
    => Task.FromResult(new GetResultResponse
                       {
                         Result = MockResult,
                       });

  /// <inheritdocs />
  [Count]
  public override async Task WatchResults(IAsyncStreamReader<WatchResultRequest>   requestStream,
                                          IServerStreamWriter<WatchResultResponse> responseStream,
                                          ServerCallContext                        context)
  {
    await foreach (var _ in requestStream.ReadAllAsync())
    {
      await responseStream.WriteAsync(new WatchResultResponse
                                      {
                                        Status = ResultStatus.Unspecified,
                                      })
                          .ConfigureAwait(false);
    }
  }
}
