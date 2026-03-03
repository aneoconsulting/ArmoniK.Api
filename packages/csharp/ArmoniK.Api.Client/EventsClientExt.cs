// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2026. All rights reserved.
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
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.Common.Exceptions;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Events;
using ArmoniK.Api.gRPC.V1.Results;
using ArmoniK.Utils;

using Grpc.Core;

using JetBrains.Annotations;

namespace ArmoniK.Api.Client
{
  /// <summary>
  ///   <see cref="Events.EventsClient" /> extensions methods
  /// </summary>
  [PublicAPI]
  public static class EventsClientExt
  {
    private static FiltersAnd ResultsFilter(string resultId)
      => new()
         {
           And =
           {
             new FilterField
             {
               Field = new ResultField
                       {
                         ResultRawField = new ResultRawField
                                          {
                                            Field = ResultRawEnumField.ResultId,
                                          },
                       },
               FilterString = new FilterString
                              {
                                Operator = FilterStringOperator.Equal,
                                Value    = resultId,
                              },
             },
           },
         };


    /// <summary>
    ///   Wait until the given results are completed
    /// </summary>
    /// <param name="client">gRPC result client</param>
    /// <param name="sessionId">The session ID in which the results are located</param>
    /// <param name="resultIds">A collection of results to wait for</param>
    /// <param name="cancellationToken">Token used to cancel the execution of the method</param>
    /// <exception cref="Exception">if a result is aborted</exception>
    [PublicAPI]
    [Obsolete("Use the overload with the bucket size and the parallelism")]
    public static Task WaitForResultsAsync(this Events.EventsClient client,
                                           string                   sessionId,
                                           ICollection<string>      resultIds,
                                           CancellationToken        cancellationToken = default)
      => client.WaitForResultsAsync(sessionId,
                                    resultIds,
                                    100,
                                    1,
                                    cancellationToken);


    /// <summary>
    ///   Wait until the given results are completed
    /// </summary>
    /// <param name="client">gRPC result client</param>
    /// <param name="sessionId">The session ID in which the results are located</param>
    /// <param name="resultIds">A collection of results to wait for</param>
    /// <param name="parallelism">Number of parallel threads to use. One bucket per thread.</param>
    /// <param name="bucket_size">Number of results Id to use to create the request to the event API</param>
    /// <param name="cancellationToken">Token used to cancel the execution of the method</param>
    /// <exception cref="Exception">if a result is aborted</exception>
    [PublicAPI]
    public static async Task WaitForResultsAsync(this Events.EventsClient client,
                                                 string                   sessionId,
                                                 ICollection<string>      resultIds,
                                                 int                      bucket_size       = 100,
                                                 int                      parallelism       = 1,
                                                 CancellationToken        cancellationToken = default)
      => await resultIds.ToChunks(bucket_size)
                        .ParallelForEach(new ParallelTaskOptions
                                         {
                                           ParallelismLimit = parallelism,
                                         },
                                         async results =>
                                         {
                                           var resultsCompleted = new List<string>();
                                           var resultsNotFound  = new HashSet<string>(results);
                                           while (resultsNotFound.Any())
                                           {
                                             using var streamingCall = client.GetEvents(new EventSubscriptionRequest
                                                                                        {
                                                                                          SessionId = sessionId,
                                                                                          ReturnedEvents =
                                                                                          {
                                                                                            EventsEnum.ResultStatusUpdate,
                                                                                            EventsEnum.NewResult,
                                                                                          },
                                                                                          ResultsFilters = new Filters
                                                                                                           {
                                                                                                             Or =
                                                                                                             {
                                                                                                               resultsNotFound.Select(ResultsFilter),
                                                                                                             },
                                                                                                           },
                                                                                        },
                                                                                        cancellationToken: cancellationToken);
                                             try
                                             {
                                               while (await streamingCall.ResponseStream.MoveNext(cancellationToken))
                                               {
                                                 var resp = streamingCall.ResponseStream.Current;
                                                 if (resp.UpdateCase == EventSubscriptionResponse.UpdateOneofCase.ResultStatusUpdate &&
                                                     resultsNotFound.Contains(resp.ResultStatusUpdate.ResultId))
                                                 {
                                                   if (resp.ResultStatusUpdate.Status == ResultStatus.Completed)
                                                   {
                                                     resultsCompleted.Add(resp.ResultStatusUpdate.ResultId);
                                                     resultsNotFound.Remove(resp.ResultStatusUpdate.ResultId);
                                                     if (!resultsNotFound.Any())
                                                     {
                                                       break;
                                                     }
                                                   }
                                                   else if (resp.ResultStatusUpdate.Status == ResultStatus.Aborted)
                                                   {
                                                     throw new ResultAbortedException($"Result {resp.ResultStatusUpdate.ResultId} has been aborted",
                                                                                      resp.ResultStatusUpdate.ResultId,
                                                                                      resultsCompleted);
                                                   }
                                                 }

                                                 if (resp.UpdateCase == EventSubscriptionResponse.UpdateOneofCase.NewResult &&
                                                     resultsNotFound.Contains(resp.NewResult.ResultId))
                                                 {
                                                   if (resp.NewResult.Status == ResultStatus.Completed)
                                                   {
                                                     resultsCompleted.Add(resp.NewResult.ResultId);
                                                     resultsNotFound.Remove(resp.NewResult.ResultId);
                                                     if (!resultsNotFound.Any())
                                                     {
                                                       break;
                                                     }
                                                   }
                                                   else if (resp.NewResult.Status == ResultStatus.Aborted)
                                                   {
                                                     throw new ResultAbortedException($"Result {resp.NewResult.ResultId} has been aborted",
                                                                                      resp.NewResult.ResultId,
                                                                                      resultsCompleted);
                                                   }
                                                 }
                                               }
                                             }
                                             catch (OperationCanceledException)
                                             {
                                             }
                                             catch (RpcException)
                                             {
                                             }
                                           }
                                         });
  }
}
