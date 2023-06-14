// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2022. All rights reserved.
//   W. Kirschenmann   <wkirschenmann@aneo.fr>
//   J. Gurhem         <jgurhem@aneo.fr>
//   D. Dubuc          <ddubuc@aneo.fr>
//   L. Ziane Khodja   <lzianekhodja@aneo.fr>
//   F. Lemaitre       <flemaitre@aneo.fr>
//   S. Djebbar        <sdjebbar@aneo.fr>
//   J. Fonseca        <jfonseca@aneo.fr>
//
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//         http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Results;

using Google.Protobuf;

using JetBrains.Annotations;

namespace ArmoniK.Api.Client
{
  /// <summary>
  ///   Extension to simplify <see cref="Results.ResultsClient" /> usage
  /// </summary>
  [PublicAPI]
  public static class ResultsClientExt
  {
    /// <summary>
    ///   Upload data to populate an existing result
    /// </summary>
    /// <param name="client">gRPC result client</param>
    /// <param name="sessionId">The id of the session</param>
    /// <param name="resultId">The id of the result</param>
    /// <param name="data">The data to send</param>
    /// <returns>
    ///   The upload result response
    /// </returns>
    [PublicAPI]
    public static async Task<UploadResultDataResponse> UploadResultData(this Results.ResultsClient client,
                                                                        string                     sessionId,
                                                                        string                     resultId,
                                                                        byte[]                     data)
    {
      var configuration = await client.GetServiceConfigurationAsync(new Empty());

      var stream = client.UploadResultData();

      await stream.RequestStream.WriteAsync(new UploadResultDataRequest
                                            {
                                              Id = new UploadResultDataRequest.Types.ResultIdentifier
                                                   {
                                                     ResultId  = resultId,
                                                     SessionId = sessionId,
                                                   },
                                            })
                  .ConfigureAwait(false);

      var start = 0;
      while (start < data.Length)
      {
        var chunkSize = Math.Min(configuration.DataChunkMaxSize,
                                 data.Length - start);

        await stream.RequestStream.WriteAsync(new UploadResultDataRequest
                                              {
                                                DataChunk = UnsafeByteOperations.UnsafeWrap(data.AsMemory()
                                                                                                .Slice(start,
                                                                                                       chunkSize)),
                                              })
                    .ConfigureAwait(false);

        start += chunkSize;
      }

      await stream.RequestStream.CompleteAsync()
                  .ConfigureAwait(false);

      return await stream.ResponseAsync.ConfigureAwait(false);
    }

    /// <summary>
    ///   Download a result
    /// </summary>
    /// <param name="client">gRPC result client</param>
    /// <param name="sessionId">The id of the session</param>
    /// <param name="resultId">The id of the result</param>
    /// <param name="cancellationToken">Token used to cancel the execution of the method</param>
    /// <returns>
    ///   A byte array containing the data associated to the result
    /// </returns>
    [PublicAPI]
    public static async Task<byte[]> DownloadResultData(this Results.ResultsClient client,
                                                        string                     sessionId,
                                                        string                     resultId,
                                                        CancellationToken          cancellationToken)
    {
      var stream = client.DownloadResultData(new DownloadResultDataRequest
                                             {
                                               ResultId  = resultId,
                                               SessionId = sessionId,
                                             });

      var result = new List<byte>();

      while (await stream.ResponseStream.MoveNext(cancellationToken))
      {
        result.AddRange(stream.ResponseStream.Current.DataChunk.ToByteArray());
      }

      return result.ToArray();
    }
  }
}
