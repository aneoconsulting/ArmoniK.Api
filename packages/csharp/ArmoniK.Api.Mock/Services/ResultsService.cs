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
using ArmoniK.Api.gRPC.V1.Results;

using Grpc.Core;

using Results = ArmoniK.Api.gRPC.V1.Results.Results;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class ResultsService : Results.ResultsBase
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
}
