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
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;
using ArmoniK.Api.gRPC.V1.Worker;
using ArmoniK.Api.Worker.Worker;

using Google.Protobuf;

using Grpc.Core;

using Microsoft.Extensions.Logging;

using NUnit.Framework;

namespace ArmoniK.Api.Worker.Tests;

[TestFixture]
public class TaskHandlerTest
{
  [SetUp]
  public void SetUp()
  {
  }

  [TearDown]
  public virtual void TearDown()
  {
  }

  private class MyAsyncStreamReader : IAsyncStreamReader<ProcessRequest>
  {
    private readonly IAsyncEnumerator<ProcessRequest> asyncEnumerator_;

    public MyAsyncStreamReader(IEnumerable<ProcessRequest> requests)
      => asyncEnumerator_ = requests.ToAsyncEnumerable()
                                    .GetAsyncEnumerator();

    public async Task<bool> MoveNext(CancellationToken cancellationToken)
      => await asyncEnumerator_.MoveNextAsync(cancellationToken)
                               .ConfigureAwait(false);

    public ProcessRequest Current
      => asyncEnumerator_.Current;
  }

  private class MyClientStreamWriter<T> : IClientStreamWriter<T>
  {
    public readonly ConcurrentBag<T> Messages = new();
    private         bool             isComplete_;

    public MyClientStreamWriter()
      => isComplete_ = false;

    public Task WriteAsync(T message)
    {
      if (isComplete_)
      {
        throw new InvalidOperationException("Stream has been completed");
      }

      Messages.Add(message);
      return Task.CompletedTask;
    }

    public WriteOptions WriteOptions { get; set; }

    public Task CompleteAsync()
    {
      isComplete_ = true;
      return Task.CompletedTask;
    }
  }

  private class MyAgent : Agent.AgentClient
  {
    private readonly MyClientStreamWriter<Result>            resultStream_;
    private readonly MyClientStreamWriter<CreateTaskRequest> taskStream_;

    public MyAgent()
    {
      resultStream_ = new MyClientStreamWriter<Result>();
      taskStream_   = new MyClientStreamWriter<CreateTaskRequest>();
    }

    public override AsyncClientStreamingCall<Result, ResultReply> SendResult(Metadata          headers           = null,
                                                                             DateTime?         deadline          = null,
                                                                             CancellationToken cancellationToken = default)
      => new(resultStream_,
             Task.FromResult(new ResultReply()),
             Task.FromResult(new Metadata()),
             () => Status.DefaultSuccess,
             () => new Metadata(),
             () =>
             {
             });

    public override AsyncClientStreamingCall<CreateTaskRequest, CreateTaskReply> CreateTask(Metadata          headers           = null,
                                                                                            DateTime?         deadline          = null,
                                                                                            CancellationToken cancellationToken = default)
      => new(taskStream_,
             Task.FromResult(new CreateTaskReply()),
             Task.FromResult(new Metadata()),
             () => Status.DefaultSuccess,
             () => new Metadata(),
             () =>
             {
             });

    public List<Result> GetResults()
      => resultStream_.Messages.ToList();

    public List<CreateTaskRequest> GetTaskRequests()
      => taskStream_.Messages.ToList();
  }


  [Test]
  [TestCaseSource(typeof(TaskHandlerTest),
                  nameof(TaskHandlerCreateShouldThrowTestCases))]
  public void TaskHandlerCreateShouldThrow(IEnumerable<ProcessRequest> requests)
  {
    var stream = new MyAsyncStreamReader(requests);

    var agent = new MyAgent();

    Assert.ThrowsAsync<InvalidOperationException>(async () => await TaskHandler.Create(stream,
                                                                                       agent,
                                                                                       new LoggerFactory(),
                                                                                       CancellationToken.None)
                                                                               .ConfigureAwait(false));
  }

  [Test]
  [TestCaseSource(typeof(TaskHandlerTest),
                  nameof(TaskHandlerCreateShouldSucceedTestCases))]
  public async Task TaskHandlerCreateShouldSucceed(IEnumerable<ProcessRequest> requests)
  {
    var stream = new MyAsyncStreamReader(requests);

    var agent = new MyAgent();

    var taskHandler = await TaskHandler.Create(stream,
                                               agent,
                                               new LoggerFactory(),
                                               CancellationToken.None)
                                       .ConfigureAwait(false);

    Assert.NotNull(taskHandler.Token);
    Assert.IsNotEmpty(taskHandler.Token);
    Assert.IsNotEmpty(taskHandler.Payload);
    Assert.IsNotEmpty(taskHandler.SessionId);
    Assert.IsNotEmpty(taskHandler.TaskId);
  }

  [Test]
  public async Task CheckTaskHandlerDataAreCorrect()
  {
    var stream = new MyAsyncStreamReader(WorkingRequest1);

    var agent = new MyAgent();

    var taskHandler = await TaskHandler.Create(stream,
                                               agent,
                                               new LoggerFactory(),
                                               CancellationToken.None)
                                       .ConfigureAwait(false);

    Assert.IsNotEmpty(taskHandler.Payload);
    Assert.AreEqual("testPayload1Payload2",
                    ByteString.CopyFrom(taskHandler.Payload)
                              .ToStringUtf8());
    Assert.AreEqual(2,
                    taskHandler.DataDependencies.Count);
    Assert.AreEqual("Data1Data2",
                    ByteString.CopyFrom(taskHandler.DataDependencies.Values.First())
                              .ToStringUtf8());
    Assert.AreEqual("Data1Data2Data2Data2",
                    ByteString.CopyFrom(taskHandler.DataDependencies.Values.Last())
                              .ToStringUtf8());
    Assert.AreEqual("TaskId",
                    taskHandler.TaskId);
    Assert.AreEqual("SessionId",
                    taskHandler.SessionId);
    Assert.AreEqual("Token",
                    taskHandler.Token);

    await taskHandler.SendResult("test",
                                 Encoding.ASCII.GetBytes("TestData"));

    var results = agent.GetResults();
    foreach (var r in results)
    {
      Console.WriteLine(r);
    }

    Assert.AreEqual(4,
                    results.Count);

    Assert.AreEqual(Result.TypeOneofCase.Init,
                    results[0]
                      .TypeCase);
    Assert.AreEqual(true,
                    results[0]
                      .Init.LastResult);

    Assert.AreEqual(Result.TypeOneofCase.Data,
                    results[1]
                      .TypeCase);
    Assert.AreEqual(true,
                    results[1]
                      .Data.DataComplete);

    Assert.AreEqual(Result.TypeOneofCase.Data,
                    results[2]
                      .TypeCase);
    Assert.AreEqual("TestData",
                    results[2]
                      .Data.Data);

    Assert.AreEqual(Result.TypeOneofCase.Init,
                    results[3]
                      .TypeCase);
    Assert.AreEqual("test",
                    results[3]
                      .Init.Key);


    await taskHandler.CreateTasksAsync(new List<TaskRequest>
                                       {
                                         new()
                                         {
                                           Payload = ByteString.CopyFromUtf8("Payload"),
                                           DataDependencies =
                                           {
                                             "DD",
                                           },
                                           ExpectedOutputKeys =
                                           {
                                             "EOK",
                                           },
                                         },
                                       });

    var tasks = agent.GetTaskRequests();
    Console.WriteLine();
    foreach (var t in tasks)
    {
      Console.WriteLine(t);
    }

    Assert.AreEqual(5,
                    tasks.Count);

    Assert.AreEqual(CreateTaskRequest.TypeOneofCase.InitTask,
                    tasks[0]
                      .TypeCase);
    Assert.AreEqual(true,
                    tasks[0]
                      .InitTask.LastTask);

    Assert.AreEqual(CreateTaskRequest.TypeOneofCase.TaskPayload,
                    tasks[1]
                      .TypeCase);
    Assert.AreEqual(true,
                    tasks[1]
                      .TaskPayload.DataComplete);

    Assert.AreEqual(CreateTaskRequest.TypeOneofCase.TaskPayload,
                    tasks[2]
                      .TypeCase);
    Assert.AreEqual("Payload",
                    tasks[2]
                      .TaskPayload.Data);

    Assert.AreEqual(CreateTaskRequest.TypeOneofCase.InitTask,
                    tasks[3]
                      .TypeCase);
    Assert.AreEqual("DD",
                    tasks[3]
                      .InitTask.Header.DataDependencies.Single());
    Assert.AreEqual("EOK",
                    tasks[3]
                      .InitTask.Header.ExpectedOutputKeys.Single());

    Assert.AreEqual(CreateTaskRequest.TypeOneofCase.InitRequest,
                    tasks[4]
                      .TypeCase);
  }

  private static readonly ProcessRequest InitData1 = new()
                                                     {
                                                       CommunicationToken = "Token",
                                                       Compute = new ProcessRequest.Types.ComputeRequest
                                                                 {
                                                                   InitData = new ProcessRequest.Types.ComputeRequest.Types.InitData
                                                                              {
                                                                                Key = "DataKey1",
                                                                              },
                                                                 },
                                                     };

  private static readonly ProcessRequest InitData2 = new()
                                                     {
                                                       CommunicationToken = "Token",
                                                       Compute = new ProcessRequest.Types.ComputeRequest
                                                                 {
                                                                   InitData = new ProcessRequest.Types.ComputeRequest.Types.InitData
                                                                              {
                                                                                Key = "DataKey2",
                                                                              },
                                                                 },
                                                     };

  private static readonly ProcessRequest LastDataTrue = new()
                                                        {
                                                          CommunicationToken = "Token",
                                                          Compute = new ProcessRequest.Types.ComputeRequest
                                                                    {
                                                                      InitData = new ProcessRequest.Types.ComputeRequest.Types.InitData
                                                                                 {
                                                                                   LastData = true,
                                                                                 },
                                                                    },
                                                        };

  private static readonly ProcessRequest LastDataFalse = new()
                                                         {
                                                           CommunicationToken = "Token",
                                                           Compute = new ProcessRequest.Types.ComputeRequest
                                                                     {
                                                                       InitData = new ProcessRequest.Types.ComputeRequest.Types.InitData
                                                                                  {
                                                                                    LastData = false,
                                                                                  },
                                                                     },
                                                         };

  private static readonly ProcessRequest InitRequestPayload = new()
                                                              {
                                                                CommunicationToken = "Token",
                                                                Compute = new ProcessRequest.Types.ComputeRequest
                                                                          {
                                                                            InitRequest = new ProcessRequest.Types.ComputeRequest.Types.InitRequest
                                                                                          {
                                                                                            Payload = new DataChunk
                                                                                                      {
                                                                                                        Data = ByteString.CopyFromUtf8("test"),
                                                                                                      },
                                                                                            Configuration = new Configuration
                                                                                                            {
                                                                                                              DataChunkMaxSize = 100,
                                                                                                            },
                                                                                            ExpectedOutputKeys =
                                                                                            {
                                                                                              "EOK",
                                                                                            },
                                                                                            SessionId = "SessionId",
                                                                                            TaskId    = "TaskId",
                                                                                          },
                                                                          },
                                                              };

  private static readonly ProcessRequest InitRequestEmptyPayload = new()
                                                                   {
                                                                     CommunicationToken = "Token",
                                                                     Compute = new ProcessRequest.Types.ComputeRequest
                                                                               {
                                                                                 InitRequest = new ProcessRequest.Types.ComputeRequest.Types.InitRequest
                                                                                               {
                                                                                                 Configuration = new Configuration
                                                                                                                 {
                                                                                                                   DataChunkMaxSize = 100,
                                                                                                                 },
                                                                                                 ExpectedOutputKeys =
                                                                                                 {
                                                                                                   "EOK",
                                                                                                 },
                                                                                                 SessionId = "SessionId",
                                                                                                 TaskId    = "TaskId",
                                                                                               },
                                                                               },
                                                                   };

  private static readonly ProcessRequest Payload1 = new()
                                                    {
                                                      CommunicationToken = "Token",
                                                      Compute = new ProcessRequest.Types.ComputeRequest
                                                                {
                                                                  Payload = new DataChunk
                                                                            {
                                                                              Data = ByteString.CopyFromUtf8("Payload1"),
                                                                            },
                                                                },
                                                    };

  private static readonly ProcessRequest Payload2 = new()
                                                    {
                                                      CommunicationToken = "Token",
                                                      Compute = new ProcessRequest.Types.ComputeRequest
                                                                {
                                                                  Payload = new DataChunk
                                                                            {
                                                                              Data = ByteString.CopyFromUtf8("Payload2"),
                                                                            },
                                                                },
                                                    };

  private static readonly ProcessRequest PayloadComplete = new()
                                                           {
                                                             CommunicationToken = "Token",
                                                             Compute = new ProcessRequest.Types.ComputeRequest
                                                                       {
                                                                         Payload = new DataChunk
                                                                                   {
                                                                                     DataComplete = true,
                                                                                   },
                                                                       },
                                                           };

  private static readonly ProcessRequest Data1 = new()
                                                 {
                                                   CommunicationToken = "Token",
                                                   Compute = new ProcessRequest.Types.ComputeRequest
                                                             {
                                                               Data = new DataChunk
                                                                      {
                                                                        Data = ByteString.CopyFromUtf8("Data1"),
                                                                      },
                                                             },
                                                 };

  private static readonly ProcessRequest Data2 = new()
                                                 {
                                                   CommunicationToken = "Token",
                                                   Compute = new ProcessRequest.Types.ComputeRequest
                                                             {
                                                               Data = new DataChunk
                                                                      {
                                                                        Data = ByteString.CopyFromUtf8("Data2"),
                                                                      },
                                                             },
                                                 };

  private static readonly ProcessRequest DataComplete = new()
                                                        {
                                                          CommunicationToken = "Token",
                                                          Compute = new ProcessRequest.Types.ComputeRequest
                                                                    {
                                                                      Data = new DataChunk
                                                                             {
                                                                               DataComplete = true,
                                                                             },
                                                                    },
                                                        };

  public static IEnumerable TaskHandlerCreateShouldThrowTestCases
  {
    get
    {
      yield return new TestCaseData(new ProcessRequest[]
                                    {
                                    }.AsEnumerable());
      yield return new TestCaseData(new[]
                                    {
                                      InitData1,
                                    }.AsEnumerable());
      yield return new TestCaseData(new[]
                                    {
                                      InitData2,
                                    }.AsEnumerable());
      yield return new TestCaseData(new[]
                                    {
                                      LastDataTrue,
                                    }.AsEnumerable());
      yield return new TestCaseData(new[]
                                    {
                                      LastDataFalse,
                                    }.AsEnumerable());
      yield return new TestCaseData(new[]
                                    {
                                      InitRequestPayload,
                                    }.AsEnumerable()).SetArgDisplayNames(nameof(InitRequestPayload));
      yield return new TestCaseData(new[]
                                    {
                                      DataComplete,
                                    }.AsEnumerable()).SetArgDisplayNames(nameof(DataComplete));
      yield return new TestCaseData(new[]
                                    {
                                      InitRequestEmptyPayload,
                                    }.AsEnumerable()).SetArgDisplayNames(nameof(InitRequestEmptyPayload));
      yield return new TestCaseData(new[]
                                    {
                                      InitRequestPayload,
                                      PayloadComplete,
                                      InitData1,
                                      Data1,
                                      LastDataTrue,
                                    }.AsEnumerable()).SetArgDisplayNames("NotWorkingRequest1");
      yield return new TestCaseData(new[]
                                    {
                                      InitRequestPayload,
                                      InitData1,
                                      Data1,
                                      DataComplete,
                                      LastDataTrue,
                                    }.AsEnumerable()).SetArgDisplayNames("NotWorkingRequest2");
      yield return new TestCaseData(new[]
                                    {
                                      InitRequestPayload,
                                      PayloadComplete,
                                      Data1,
                                      DataComplete,
                                      LastDataTrue,
                                    }.AsEnumerable()).SetArgDisplayNames("NotWorkingRequest3");
    }
  }

  private static readonly IEnumerable<ProcessRequest> WorkingRequest1 = new[]
                                                                        {
                                                                          InitRequestPayload,
                                                                          Payload1,
                                                                          Payload2,
                                                                          PayloadComplete,
                                                                          InitData1,
                                                                          Data1,
                                                                          Data2,
                                                                          DataComplete,
                                                                          InitData2,
                                                                          Data1,
                                                                          Data2,
                                                                          Data2,
                                                                          Data2,
                                                                          DataComplete,
                                                                          LastDataTrue,
                                                                        }.AsEnumerable();

  private static readonly IEnumerable<ProcessRequest> WorkingRequest2 = new[]
                                                                        {
                                                                          InitRequestPayload,
                                                                          Payload1,
                                                                          PayloadComplete,
                                                                          InitData1,
                                                                          Data1,
                                                                          DataComplete,
                                                                          LastDataTrue,
                                                                        }.AsEnumerable();

  private static readonly IEnumerable<ProcessRequest> WorkingRequest3 = new[]
                                                                        {
                                                                          InitRequestPayload,
                                                                          PayloadComplete,
                                                                          InitData1,
                                                                          Data1,
                                                                          DataComplete,
                                                                          LastDataTrue,
                                                                        }.AsEnumerable();

  public static IEnumerable TaskHandlerCreateShouldSucceedTestCases
  {
    get
    {
      yield return new TestCaseData(WorkingRequest1).SetArgDisplayNames(nameof(WorkingRequest1));
      yield return new TestCaseData(WorkingRequest2).SetArgDisplayNames(nameof(WorkingRequest2));
      yield return new TestCaseData(WorkingRequest3).SetArgDisplayNames(nameof(WorkingRequest3));
    }
  }
}
