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
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;
using ArmoniK.Api.gRPC.V1.Worker;

using Microsoft.Extensions.Logging;

namespace ArmoniK.Api.Worker.Worker;

internal class ReadFromFolderDict : IReadOnlyDictionary<string, byte[]>
{
  private readonly Dictionary<string, byte[]> data_ = new();
  private readonly IList<string>              dataDependencies_;
  private readonly string                     folder_;

  public ReadFromFolderDict(string        folder,
                            IList<string> dataDependencies)
  {
    folder_           = folder;
    dataDependencies_ = dataDependencies;
  }

  /// <inheritdoc />
  public IEnumerator<KeyValuePair<string, byte[]>> GetEnumerator()
    => dataDependencies_.Select(key => new KeyValuePair<string, byte[]>(key,
                                                                        this[key]))
                        .GetEnumerator();

  IEnumerator IEnumerable.GetEnumerator()
    => GetEnumerator();

  /// <inheritdoc />
  public int Count
    => dataDependencies_.Count;

  /// <inheritdoc />
  public bool ContainsKey(string key)
    => dataDependencies_.Contains(key);

  /// <inheritdoc />
  public bool TryGetValue(string                            key,
                          [MaybeNullWhen(false)] out byte[] value)
  {
    var r = ContainsKey(key);
    if (r)
    {
      value = this[key];
      return r;
    }

    value = null;
    return r;
  }

  /// <inheritdoc />
  public byte[] this[string key]
  {
    get
    {
      if (data_.TryGetValue(key,
                            out var value))
      {
        return value;
      }

      var bytes = File.ReadAllBytes(Path.Combine(folder_,
                                                 key));
      data_.Add(key,
                bytes);
      return bytes;
    }
  }

  /// <inheritdoc />
  public IEnumerable<string> Keys
    => dataDependencies_;

  /// <inheritdoc />
  public IEnumerable<byte[]> Values
    => dataDependencies_.Select(key => this[key]);
}

/// <summary>
///   Task handler that unifies task execution and calls to the Agent
/// </summary>
public class TaskHandler : ITaskHandler
{
  private readonly CancellationToken    cancellationToken_;
  private readonly Agent.AgentClient    client_;
  private readonly string               folder_;
  private readonly ILogger<TaskHandler> logger_;
  private readonly ILoggerFactory       loggerFactory_;


  /// <summary>
  ///   Instantiate task handler that unifies task execution and calls to the Agent
  /// </summary>
  /// <param name="processRequest">Task execution request</param>
  /// <param name="client">Client to the agent</param>
  /// <param name="loggerFactory">Logger factory used to create loggers</param>
  /// <param name="cancellationToken">Token used to cancel the execution of the method</param>
  /// <exception cref="InvalidOperationException">when payload is not found</exception>
  public TaskHandler(ProcessRequest    processRequest,
                     Agent.AgentClient client,
                     ILoggerFactory    loggerFactory,
                     CancellationToken cancellationToken)
  {
    client_            = client;
    cancellationToken_ = cancellationToken;
    loggerFactory_     = loggerFactory;
    logger_            = loggerFactory.CreateLogger<TaskHandler>();
    folder_            = processRequest.DataFolder;

    Token           = processRequest.CommunicationToken;
    SessionId       = processRequest.SessionId;
    TaskId          = processRequest.TaskId;
    TaskOptions     = processRequest.TaskOptions;
    ExpectedResults = processRequest.ExpectedOutputKeys;
    DataDependencies = new ReadFromFolderDict(processRequest.DataFolder,
                                              processRequest.DataDependencies);
    Configuration = processRequest.Configuration;


    try
    {
      Payload = File.ReadAllBytes(Path.Combine(processRequest.DataFolder,
                                               processRequest.PayloadId));
    }
    catch (ArgumentException e)
    {
      throw new InvalidOperationException("Payload not found",
                                          e);
    }
  }

  /// <summary>
  ///   Communication token used to identify requests
  /// </summary>
  public string Token { get; }

  /// <inheritdoc />
  public Configuration Configuration { get; }

  /// <inheritdoc />
  public string SessionId { get; }

  /// <inheritdoc />
  public string TaskId { get; }

  /// <inheritdoc />
  public TaskOptions TaskOptions { get; }

  /// <inheritdoc />
  public byte[] Payload { get; }

  /// <inheritdoc />
  public IReadOnlyDictionary<string, byte[]> DataDependencies { get; }

  /// <inheritdoc />
  public IList<string> ExpectedResults { get; }

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
  public async Task<CreateResultsMetaDataResponse> CreateResultsMetaDataAsync(IEnumerable<CreateResultsMetaDataRequest.Types.ResultCreate> results)
    => await client_.CreateResultsMetaDataAsync(new CreateResultsMetaDataRequest
                                                {
                                                  CommunicationToken = Token,
                                                  Results =
                                                  {
                                                    results,
                                                  },
                                                  SessionId = SessionId,
                                                })
                    .ConfigureAwait(false);


  /// <inheritdoc />
  public async Task SendResult(string key,
                               byte[] data)
  {
    await using (var fs = new FileStream(Path.Combine(folder_,
                                                      key),
                                         FileMode.OpenOrCreate))
    {
      await using var w = new BinaryWriter(fs);
      w.Write(data);
    }

    await client_.NotifyResultDataAsync(new NotifyResultDataRequest
                                        {
                                          CommunicationToken = Token,
                                          Ids =
                                          {
                                            new NotifyResultDataRequest.Types.ResultIdentifier
                                            {
                                              SessionId = SessionId,
                                              ResultId  = key,
                                            },
                                          },
                                        })
                 .ConfigureAwait(false);
  }

  /// <inheritdoc />
  public ValueTask DisposeAsync()
    => ValueTask.CompletedTask;

  /// <inheritdoc />
  public async Task<SubmitTasksResponse> SubmitTasksAsync(IEnumerable<SubmitTasksRequest.Types.TaskCreation> taskCreations,
                                                          TaskOptions?                                       submissionTaskOptions)
    => await client_.SubmitTasksAsync(new SubmitTasksRequest
                                      {
                                        CommunicationToken = Token,
                                        SessionId          = SessionId,
                                        TaskCreations =
                                        {
                                          taskCreations,
                                        },
                                        TaskOptions = submissionTaskOptions,
                                      })
                    .ConfigureAwait(false);

  /// <inheritdoc />
  public async Task<CreateResultsResponse> CreateResultsAsync(IEnumerable<CreateResultsRequest.Types.ResultCreate> results)
    => await client_.CreateResultsAsync(new CreateResultsRequest
                                        {
                                          CommunicationToken = Token,
                                          SessionId          = SessionId,
                                          Results =
                                          {
                                            results,
                                          },
                                        })
                    .ConfigureAwait(false);
}
