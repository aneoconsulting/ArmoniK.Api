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
using System.Linq;
using System.Runtime.CompilerServices;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Submitter;

using Google.Protobuf;

using JetBrains.Annotations;

namespace ArmoniK.Api.Client.Submitter
{
  /// <summary>
  ///   Extension to simplify <see cref="gRPC.V1.Submitter.Submitter.SubmitterClient" /> usage
  /// </summary>
  [PublicAPI]
  public static class SubmitterClientExt
  {
    /// <summary>
    ///   Create task request without streaming
    /// </summary>
    /// <param name="client">gRPC client to the Submitter</param>
    /// <param name="sessionId">Id of the sessions</param>
    /// <param name="taskOptions">Task Options for the tasks in this request</param>
    /// <param name="taskRequests">The collection of request</param>
    /// <param name="cancellationToken">Token used to cancel the execution of the method</param>
    /// <returns>
    ///   The reply to task creation
    /// </returns>
    public static async Task<CreateTaskReply> CreateTasksAsync(this gRPC.V1.Submitter.Submitter.SubmitterClient client,
                                                               string                                           sessionId,
                                                               TaskOptions?                                     taskOptions,
                                                               IEnumerable<TaskRequest>                         taskRequests,
                                                               CancellationToken                                cancellationToken = default)
      => await CreateTasksAsync(client,
                                sessionId,
                                taskOptions,
                                taskRequests.ToAsyncEnumerable(),
                                cancellationToken)
           .ConfigureAwait(false);

    /// <summary>
    ///   Create task request without streaming
    /// </summary>
    /// <param name="client">gRPC client to the Submitter</param>
    /// <param name="sessionId">Id of the sessions</param>
    /// <param name="taskOptions">Task Options for the tasks in this request</param>
    /// <param name="taskRequests">The collection of request</param>
    /// <param name="cancellationToken">Token used to cancel the execution of the method</param>
    /// <returns>
    ///   The reply to task creation
    /// </returns>
    public static async Task<CreateTaskReply> CreateTasksAsync(this gRPC.V1.Submitter.Submitter.SubmitterClient client,
                                                               string                                           sessionId,
                                                               TaskOptions?                                     taskOptions,
                                                               IAsyncEnumerable<TaskRequest>                    taskRequests,
                                                               CancellationToken                                cancellationToken = default)
    {
      var serviceConfiguration = await client.GetServiceConfigurationAsync(new Empty(),
                                                                           cancellationToken: cancellationToken);

      using var stream = client.CreateLargeTasks(cancellationToken: cancellationToken);

      await foreach (var createLargeTaskRequest in taskRequests.ToRequestStream(sessionId,
                                                                                taskOptions,
                                                                                serviceConfiguration.DataChunkMaxSize,
                                                                                cancellationToken))
      {
        await stream.RequestStream.WriteAsync(createLargeTaskRequest)
                    .ConfigureAwait(false);
      }

      await stream.RequestStream.CompleteAsync()
                  .ConfigureAwait(false);

      return await stream.ResponseAsync.ConfigureAwait(false);
    }


    private static async IAsyncEnumerable<CreateLargeTaskRequest> ToRequestStream(this IAsyncEnumerable<TaskRequest>         taskRequests,
                                                                                  string                                     sessionId,
                                                                                  TaskOptions?                               taskOptions,
                                                                                  int                                        chunkMaxSize,
                                                                                  [EnumeratorCancellation] CancellationToken cancellationToken)
    {
      yield return new CreateLargeTaskRequest
                   {
                     InitRequest = new CreateLargeTaskRequest.Types.InitRequest
                                   {
                                     SessionId   = sessionId,
                                     TaskOptions = taskOptions,
                                   },
                   };

      await using var taskRequestEnumerator = taskRequests.GetAsyncEnumerator(cancellationToken);

      if (!await taskRequestEnumerator.MoveNextAsync(cancellationToken))
      {
        yield break;
      }

      var currentRequest = taskRequestEnumerator.Current;

      while (await taskRequestEnumerator.MoveNextAsync(cancellationToken))
      {
        await foreach (var createLargeTaskRequest in currentRequest.ToRequestStream(false,
                                                                                    chunkMaxSize,
                                                                                    cancellationToken))
        {
          yield return createLargeTaskRequest;
        }


        currentRequest = taskRequestEnumerator.Current;
      }

      await foreach (var createLargeTaskRequest in currentRequest.ToRequestStream(true,
                                                                                  chunkMaxSize,
                                                                                  cancellationToken))
      {
        yield return createLargeTaskRequest;
      }
    }

#pragma warning disable CS1998
    private static async IAsyncEnumerable<CreateLargeTaskRequest> ToRequestStream(this TaskRequest taskRequest,
#pragma warning restore CS1998
                                                                                  bool                                       isLast,
                                                                                  int                                        chunkMaxSize,
                                                                                  [EnumeratorCancellation] CancellationToken cancellationToken)
    {
      yield return new CreateLargeTaskRequest
                   {
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
      if (cancellationToken.IsCancellationRequested)
      {
        yield break;
      }

      if (taskRequest.Payload.Length == 0)
      {
        yield return new CreateLargeTaskRequest
                     {
                       TaskPayload = new DataChunk
                                     {
                                       Data = ByteString.Empty,
                                     },
                     };
      }

      while (start < taskRequest.Payload.Length)
      {
        if (cancellationToken.IsCancellationRequested)
        {
          yield break;
        }

        var chunkSize = Math.Min(chunkMaxSize,
                                 taskRequest.Payload.Length - start);

        yield return new CreateLargeTaskRequest
                     {
                       TaskPayload = new DataChunk
                                     {
                                       Data = ByteString.CopyFrom(taskRequest.Payload.Span.Slice(start,
                                                                                                 chunkSize)),
                                     },
                     };

        start += chunkSize;
      }

      if (cancellationToken.IsCancellationRequested)
      {
        yield break;
      }

      yield return new CreateLargeTaskRequest
                   {
                     TaskPayload = new DataChunk
                                   {
                                     DataComplete = true,
                                   },
                   };

      if (isLast)
      {
        yield return new CreateLargeTaskRequest
                     {
                       InitTask = new InitTaskRequest
                                  {
                                    LastTask = true,
                                  },
                     };
      }
    }

    /// <summary>
    ///   Get result without streaming
    /// </summary>
    /// <param name="client">gRPC client to the Submitter</param>
    /// <param name="resultRequest">Request for result</param>
    /// <param name="cancellationToken">Token used to cancel the execution of the method</param>
    /// <returns>
    ///   A byte array containing the data associated to the result
    /// </returns>
    /// <exception cref="Exception">a result reply chunk is not data, rending it impossible to reconstitute the data</exception>
    /// <exception cref="ArgumentOutOfRangeException">result reply type is unknown</exception>
    public static async Task<byte[]> GetResultAsync(this gRPC.V1.Submitter.Submitter.SubmitterClient client,
                                                    ResultRequest                                    resultRequest,
                                                    CancellationToken                                cancellationToken = default)
    {
      var streamingCall = client.TryGetResultStream(resultRequest,
                                                    cancellationToken: cancellationToken);

      var result = new List<byte>();

      while (await streamingCall.ResponseStream.MoveNext(cancellationToken))
      {
        var reply = streamingCall.ResponseStream.Current;
        switch (reply.TypeCase)
        {
          case ResultReply.TypeOneofCase.Result:
            if (!reply.Result.DataComplete)
            {
              result.AddRange(reply.Result.Data.ToByteArray());
            }

            break;
          case ResultReply.TypeOneofCase.None:
            throw new Exception("Issue with Server !");
          case ResultReply.TypeOneofCase.Error:
            throw new Exception($"Error in task {reply.Error.TaskId}");
          case ResultReply.TypeOneofCase.NotCompletedTask:
            throw new Exception($"Task {reply.NotCompletedTask} not completed");
          default:
            throw new ArgumentOutOfRangeException();
        }
      }

      return result.ToArray();
    }
  }
}
