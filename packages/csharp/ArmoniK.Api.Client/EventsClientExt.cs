// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2024. All rights reserved.
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

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.Common.Exceptions;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Events;
using ArmoniK.Api.gRPC.V1.Results;

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
    public static async Task WaitForResultsAsync(this Events.EventsClient client,
                                                 string                   sessionId,
                                                 ICollection<string>      resultIds,
                                                 CancellationToken        cancellationToken)
    {
      var resultsNotFound = new HashSet<string>(resultIds);
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
            if (resp.UpdateCase == EventSubscriptionResponse.UpdateOneofCase.ResultStatusUpdate && resultsNotFound.Contains(resp.ResultStatusUpdate.ResultId))
            {
              if (resp.ResultStatusUpdate.Status == ResultStatus.Completed)
              {
                resultsNotFound.Remove(resp.ResultStatusUpdate.ResultId);
                if (!resultsNotFound.Any())
                {
                  break;
                }
              }

              if (resp.ResultStatusUpdate.Status == ResultStatus.Aborted)
              {
                throw new ResultAbortedException($"Result {resp.ResultStatusUpdate.ResultId} has been aborted");
              }
            }

            if (resp.UpdateCase == EventSubscriptionResponse.UpdateOneofCase.NewResult && resultsNotFound.Contains(resp.NewResult.ResultId))
            {
              if (resp.NewResult.Status == ResultStatus.Completed)
              {
                resultsNotFound.Remove(resp.NewResult.ResultId);
                if (!resultsNotFound.Any())
                {
                  break;
                }
              }

              if (resp.NewResult.Status == ResultStatus.Aborted)
              {
                throw new ResultAbortedException($"Result {resp.NewResult.ResultId} has been aborted");
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
    }
  }
}
