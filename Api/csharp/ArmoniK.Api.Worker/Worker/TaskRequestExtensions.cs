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

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;

using Google.Protobuf;

using JetBrains.Annotations;

namespace ArmoniK.Api.Worker.Worker;

[PublicAPI]
public static class TaskRequestExtensions
{
  public static IEnumerable<CreateTaskRequest> ToRequestStream(this IEnumerable<TaskRequest> taskRequests,
                                                               TaskOptions?                  taskOptions,
                                                               string                        token,
                                                               int                           chunkMaxSize)
  {
    if (taskOptions is not null)
    {
      yield return new CreateTaskRequest
                   {
                     CommunicationToken = token,
                     InitRequest = new CreateTaskRequest.Types.InitRequest
                                   {
                                     TaskOptions = taskOptions,
                                   },
                   };
    }
    else
    {
      yield return new CreateTaskRequest
                   {
                     CommunicationToken = token,
                     InitRequest        = new CreateTaskRequest.Types.InitRequest(),
                   };
    }

    using var taskRequestEnumerator = taskRequests.GetEnumerator();

    if (!taskRequestEnumerator.MoveNext())
    {
      yield break;
    }

    var currentRequest = taskRequestEnumerator.Current;

    while (taskRequestEnumerator.MoveNext())
    {
      foreach (var createLargeTaskRequest in currentRequest.ToRequestStream(false,
                                                                            token,
                                                                            chunkMaxSize))
      {
        yield return createLargeTaskRequest;
      }


      currentRequest = taskRequestEnumerator.Current;
    }

    foreach (var createLargeTaskRequest in currentRequest.ToRequestStream(true,
                                                                          token,
                                                                          chunkMaxSize))
    {
      yield return createLargeTaskRequest;
    }
  }

  public static IEnumerable<CreateTaskRequest> ToRequestStream(this TaskRequest taskRequest,
                                                               bool             isLast,
                                                               string           token,
                                                               int              chunkMaxSize)
  {
    yield return new CreateTaskRequest
                 {
                   CommunicationToken = token,
                   InitTask = new InitTaskRequest
                              {
                                Header = new TaskRequestHeader
                                         {
                                           DataDependencies =
                                           {
                                             taskRequest.DataDependencies,
                                           },
                                           ExpectedOutputKeys =
                                           {
                                             taskRequest.ExpectedOutputKeys,
                                           },
                                         },
                              },
                 };

    var start = 0;

    while (start < taskRequest.Payload.Length)
    {
      var chunkSize = Math.Min(chunkMaxSize,
                               taskRequest.Payload.Length - start);

      yield return new CreateTaskRequest
                   {
                     CommunicationToken = token,
                     TaskPayload = new DataChunk
                                   {
                                     Data = ByteString.CopyFrom(taskRequest.Payload.Span.Slice(start,
                                                                                               chunkSize)),
                                   },
                   };

      start += chunkSize;
    }

    yield return new CreateTaskRequest
                 {
                   CommunicationToken = token,
                   TaskPayload = new DataChunk
                                 {
                                   DataComplete = true,
                                 },
                 };

    if (isLast)
    {
      yield return new CreateTaskRequest
                   {
                     CommunicationToken = token,
                     InitTask = new InitTaskRequest
                                {
                                  LastTask = true,
                                },
                   };
    }
  }
}
