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
using ArmoniK.Api.gRPC.V1.Results;

using Grpc.Core;

using Results = ArmoniK.Api.gRPC.V1.Results.Results;

namespace ArmoniK.Api.Mock.Services;

public class ResultsService : Results.ResultsBase, ICountingService
{
  private static readonly ResultRaw MockResult = new()
                                                 {
                                                   SessionId   = "session-id",
                                                   ResultId    = "result-id",
                                                   Name        = "result-name",
                                                   Status      = ResultStatus.Completed,
                                                   OwnerTaskId = "owner-task-id",
                                                 };

  private CallCount calls_ = new();

  /// <inheritdocs />
  public ICounter GetCounter()
    => calls_;


  /// <inheritdocs />
  public override Task<GetOwnerTaskIdResponse> GetOwnerTaskId(GetOwnerTaskIdRequest request,
                                                              ServerCallContext     context)
  {
    Interlocked.Add(ref calls_.GetOwnerTaskId,
                    1);
    return Task.FromResult(new GetOwnerTaskIdResponse
                           {
                             SessionId = "session-id",
                           });
  }

  /// <inheritdocs />
  public override Task<ListResultsResponse> ListResults(ListResultsRequest request,
                                                        ServerCallContext  context)
  {
    Interlocked.Add(ref calls_.ListResults,
                    1);
    return Task.FromResult(new ListResultsResponse
                           {
                             Page     = 0,
                             Total    = 0,
                             PageSize = request.PageSize,
                           });
  }

  /// <inheritdocs />
  public override Task<CreateResultsMetaDataResponse> CreateResultsMetaData(CreateResultsMetaDataRequest request,
                                                                            ServerCallContext            context)
  {
    Interlocked.Add(ref calls_.CreateResultsMetaData,
                    1);
    return Task.FromResult(new CreateResultsMetaDataResponse());
  }

  /// <inheritdocs />
  public override Task<CreateResultsResponse> CreateResults(CreateResultsRequest request,
                                                            ServerCallContext    context)
  {
    Interlocked.Add(ref calls_.CreateResults,
                    1);
    return Task.FromResult(new CreateResultsResponse());
  }

  /// <inheritdocs />
  public override Task<DeleteResultsDataResponse> DeleteResultsData(DeleteResultsDataRequest request,
                                                                    ServerCallContext        context)
  {
    Interlocked.Add(ref calls_.DeleteResultsData,
                    1);
    return Task.FromResult(new DeleteResultsDataResponse
                           {
                             SessionId = "session-id",
                           });
  }

  /// <inheritdocs />
  public override Task DownloadResultData(DownloadResultDataRequest                       request,
                                          IServerStreamWriter<DownloadResultDataResponse> responseStream,
                                          ServerCallContext                               context)
  {
    Interlocked.Add(ref calls_.DownloadResultData,
                    1);
    return Task.CompletedTask;
  }

  /// <inheritdocs />
  public override Task<ResultsServiceConfigurationResponse> GetServiceConfiguration(Empty             request,
                                                                                    ServerCallContext context)
  {
    Interlocked.Add(ref calls_.GetServiceConfiguration,
                    1);
    return Task.FromResult(new ResultsServiceConfigurationResponse
                           {
                             DataChunkMaxSize = 80 * 1024,
                           });
  }


  /// <inheritdocs />
  public override async Task<UploadResultDataResponse> UploadResultData(IAsyncStreamReader<UploadResultDataRequest> requestStream,
                                                                        ServerCallContext                           context)
  {
    Interlocked.Add(ref calls_.UploadResultData,
                    1);

    await foreach (var _ in requestStream.ReadAllAsync())
    {
    }

    return new UploadResultDataResponse
           {
             Result = MockResult,
           };
  }

  /// <inheritdocs />
  public override Task<GetResultResponse> GetResult(GetResultRequest  request,
                                                    ServerCallContext context)
  {
    Interlocked.Add(ref calls_.GetResult,
                    1);

    return Task.FromResult(new GetResultResponse
                           {
                             Result = MockResult,
                           });
  }

  private struct CallCount : ICounter
  {
    public int GetOwnerTaskId          = 0;
    public int ListResults             = 0;
    public int CreateResultsMetaData   = 0;
    public int CreateResults           = 0;
    public int DeleteResultsData       = 0;
    public int DownloadResultData      = 0;
    public int GetServiceConfiguration = 0;
    public int UploadResultData        = 0;
    public int GetResult               = 0;

    public CallCount()
    {
    }
  }
}
