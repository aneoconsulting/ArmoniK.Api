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
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;
using ArmoniK.Api.gRPC.V1.Worker;

using Google.Protobuf;

using Grpc.Core;

using Microsoft.Extensions.Logging;

namespace ArmoniK.Api.Worker.Worker;

public class TaskHandler : ITaskHandler
{
  private readonly CancellationToken    cancellationToken_;
  private readonly Agent.AgentClient    client_;
  private readonly ILogger<TaskHandler> logger_;
  private readonly ILoggerFactory       loggerFactory_;

  private readonly IAsyncStreamReader<ProcessRequest> requestStream_;

  private IReadOnlyDictionary<string, byte[]>? dataDependencies_;
  private IList<string>?                       expectedResults_;

  private bool isInitialized_;

  private byte[]?      payload_;
  private string?      sessionId_;
  private string?      taskId_;
  private TaskOptions? taskOptions_;
  private string?      token_;


  private TaskHandler(IAsyncStreamReader<ProcessRequest> requestStream,
                      Agent.AgentClient                  client,
                      CancellationToken                  cancellationToken,
                      ILoggerFactory                     loggerFactory)
  {
    requestStream_     = requestStream;
    client_            = client;
    cancellationToken_ = cancellationToken;
    loggerFactory_     = loggerFactory;
    logger_            = loggerFactory.CreateLogger<TaskHandler>();
  }

  public string Token
    => token_ ?? throw TaskHandlerException(nameof(Token));

  /// <inheritdoc />
  public string SessionId
    => sessionId_ ?? throw TaskHandlerException(nameof(SessionId));

  /// <inheritdoc />
  public string TaskId
    => taskId_ ?? throw TaskHandlerException(nameof(TaskId));

  /// <inheritdoc />
  public TaskOptions TaskOptions
    => taskOptions_ ?? throw TaskHandlerException(nameof(TaskOptions));

  /// <inheritdoc />
  public byte[] Payload
    => payload_ ?? throw TaskHandlerException(nameof(Payload));

  /// <inheritdoc />
  public IReadOnlyDictionary<string, byte[]> DataDependencies
    => dataDependencies_ ?? throw TaskHandlerException(nameof(DataDependencies));

  /// <inheritdoc />
  public IList<string> ExpectedResults
    => expectedResults_ ?? throw TaskHandlerException(nameof(ExpectedResults));

  // this ? was added due to the initialization pattern with the Create method
  /// <inheritdoc />
  public Configuration? Configuration { get; private set; }

  /// <inheritdoc />
  public async Task<CreateTaskReply> CreateTasksAsync(IEnumerable<TaskRequest> tasks,
                                                      TaskOptions?             taskOptions = null)
  {
    using var stream = client_.CreateTask();

    foreach (var createLargeTaskRequest in tasks.ToRequestStream(taskOptions,
                                                                 Token,
                                                                 Configuration!.DataChunkMaxSize))
    {
      await stream.RequestStream.WriteAsync(createLargeTaskRequest)
                  .ConfigureAwait(false);
    }

    await stream.RequestStream.CompleteAsync()
                .ConfigureAwait(false);

    return await stream.ResponseAsync.ConfigureAwait(false);
  }

  /// <inheritdoc />
  public Task<byte[]> RequestResource(string key)
    => throw new NotImplementedException();

  /// <inheritdoc />
  public Task<byte[]> RequestCommonData(string key)
    => throw new NotImplementedException();

  /// <inheritdoc />
  public Task<byte[]> RequestDirectData(string key)
    => throw new NotImplementedException();

  /// <inheritdoc />
  public async Task SendResult(string key,
                               byte[] data)
  {
    using var stream = client_.SendResult();

    await stream.RequestStream.WriteAsync(new Result
                                          {
                                            CommunicationToken = Token,
                                            Init = new InitKeyedDataStream
                                                   {
                                                     Key = key,
                                                   },
                                          })
                .ConfigureAwait(false);
    var start = 0;

    while (start < data.Length)
    {
      var chunkSize = Math.Min(Configuration!.DataChunkMaxSize,
                               data.Length - start);

      await stream.RequestStream.WriteAsync(new Result
                                            {
                                              CommunicationToken = Token,
                                              Data = new DataChunk
                                                     {
                                                       Data = ByteString.CopyFrom(data.AsMemory()
                                                                                      .Span.Slice(start,
                                                                                                  chunkSize)),
                                                     },
                                            })
                  .ConfigureAwait(false);

      start += chunkSize;
    }

    await stream.RequestStream.WriteAsync(new Result
                                          {
                                            CommunicationToken = Token,
                                            Data = new DataChunk
                                                   {
                                                     DataComplete = true,
                                                   },
                                          })
                .ConfigureAwait(false);

    await stream.RequestStream.WriteAsync(new Result
                                          {
                                            CommunicationToken = Token,
                                            Init = new InitKeyedDataStream
                                                   {
                                                     LastResult = true,
                                                   },
                                          })
                .ConfigureAwait(false);

    await stream.RequestStream.CompleteAsync()
                .ConfigureAwait(false);

    var reply = await stream.ResponseAsync.ConfigureAwait(false);
    if (reply.TypeCase == ResultReply.TypeOneofCase.Error)
    {
      logger_.LogError(reply.Error);
      throw new InvalidOperationException($"Cannot send result id={key}");
    }
  }

  public ValueTask DisposeAsync()
    => ValueTask.CompletedTask;

  public static async Task<TaskHandler> Create(IAsyncStreamReader<ProcessRequest> requestStream,
                                               Agent.AgentClient                  agentClient,
                                               ILoggerFactory                     loggerFactory,
                                               CancellationToken                  cancellationToken)
  {
    var output = new TaskHandler(requestStream,
                                 agentClient,
                                 cancellationToken,
                                 loggerFactory);
    await output.Init()
                .ConfigureAwait(false);
    return output;
  }

  private async Task Init()
  {
    if (!await requestStream_.MoveNext()
                             .ConfigureAwait(false))
    {
      throw new InvalidOperationException("Request stream ended unexpectedly.");
    }

    if (requestStream_.Current.Compute.TypeCase != ProcessRequest.Types.ComputeRequest.TypeOneofCase.InitRequest)
    {
      throw new InvalidOperationException("Expected a Compute request type with InitRequest to start the stream.");
    }

    var initRequest = requestStream_.Current.Compute.InitRequest;
    sessionId_       = initRequest.SessionId;
    taskId_          = initRequest.TaskId;
    taskOptions_     = initRequest.TaskOptions;
    expectedResults_ = initRequest.ExpectedOutputKeys;
    Configuration    = initRequest.Configuration;
    token_           = requestStream_.Current.CommunicationToken;


    if (initRequest.Payload.DataComplete)
    {
      payload_ = initRequest.Payload.Data.ToByteArray();
    }
    else
    {
      var chunks    = new List<ByteString>();
      var dataChunk = initRequest.Payload;

      chunks.Add(dataChunk.Data);

      while (!dataChunk.DataComplete)
      {
        if (!await requestStream_.MoveNext(cancellationToken_)
                                 .ConfigureAwait(false))
        {
          throw new InvalidOperationException("Request stream ended unexpectedly.");
        }

        if (requestStream_.Current.Compute.TypeCase != ProcessRequest.Types.ComputeRequest.TypeOneofCase.Payload)
        {
          throw new InvalidOperationException("Expected a Compute request type with Payload to continue the stream.");
        }

        dataChunk = requestStream_.Current.Compute.Payload;

        chunks.Add(dataChunk.Data);
      }


      var size = chunks.Sum(s => s.Length);

      var payload = new byte[size];

      var start = 0;

      foreach (var chunk in chunks)
      {
        chunk.CopyTo(payload,
                     start);
        start += chunk.Length;
      }

      payload_ = payload;
    }

    var dataDependencies = new Dictionary<string, byte[]>();

    ProcessRequest.Types.ComputeRequest.Types.InitData initData;
    do
    {
      if (!await requestStream_.MoveNext(cancellationToken_)
                               .ConfigureAwait(false))
      {
        throw new InvalidOperationException("Request stream ended unexpectedly.");
      }


      if (requestStream_.Current.Compute.TypeCase != ProcessRequest.Types.ComputeRequest.TypeOneofCase.InitData)
      {
        throw new InvalidOperationException("Expected a Compute request type with InitData to continue the stream.");
      }

      initData = requestStream_.Current.Compute.InitData;
      if (!string.IsNullOrEmpty(initData.Key))
      {
        var chunks = new List<ByteString>();

        while (true)
        {
          if (!await requestStream_.MoveNext(cancellationToken_)
                                   .ConfigureAwait(false))
          {
            throw new InvalidOperationException("Request stream ended unexpectedly.");
          }

          if (requestStream_.Current.Compute.TypeCase != ProcessRequest.Types.ComputeRequest.TypeOneofCase.Data)
          {
            throw new InvalidOperationException("Expected a Compute request type with Data to continue the stream.");
          }

          var dataChunk = requestStream_.Current.Compute.Data;

          if (dataChunk.TypeCase == DataChunk.TypeOneofCase.Data)
          {
            chunks.Add(dataChunk.Data);
          }

          if (dataChunk.TypeCase == DataChunk.TypeOneofCase.None)
          {
            throw new InvalidOperationException("Expected a Compute request type with a DataChunk Payload to continue the stream.");
          }

          if (dataChunk.TypeCase == DataChunk.TypeOneofCase.DataComplete)
          {
            break;
          }
        }

        var size = chunks.Sum(s => s.Length);

        var data = new byte[size];

        var start = 0;

        foreach (var chunk in chunks)
        {
          chunk.CopyTo(data,
                       start);
          start += chunk.Length;
        }

        dataDependencies[initData.Key] = data;
      }
    } while (!string.IsNullOrEmpty(initData.Key));

    dataDependencies_ = dataDependencies;
    isInitialized_    = true;
  }

  private Exception TaskHandlerException(string argumentName)
    => isInitialized_
         ? new InvalidOperationException($"Error in initalization: {argumentName} is null")
         : new InvalidOperationException("");
}
