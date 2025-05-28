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
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;
using ArmoniK.Api.gRPC.V1.Worker;
using ArmoniK.Api.Worker.Worker;

using Grpc.Core;

using Microsoft.Extensions.Logging;

using NUnit.Framework;

namespace ArmoniK.Api.Worker.Tests;

[TestFixture]
public class TaskHandlerTest
{
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
    private readonly MyClientStreamWriter<CreateTaskRequest> taskStream_ = new();


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

    public List<CreateTaskRequest> GetTaskRequests()
      => taskStream_.Messages.ToList();
  }


  [Test]
  [TestCaseSource(typeof(TaskHandlerTest),
                  nameof(InvalidRequests))]
  public void NewTaskHandlerShouldThrow(ProcessRequest request)
  {
    var agent = new MyAgent();

    Assert.That(() => new TaskHandler(request,
                                      agent,
                                      new LoggerFactory(),
                                      CancellationToken.None),
                Throws.InstanceOf<InvalidOperationException>());
  }

  public static IEnumerable InvalidRequests
  {
    get { yield return new TestCaseData(new ProcessRequest()).SetArgDisplayNames("Empty request"); }
  }

  [Test]
  public async Task NewTaskHandlerShouldSucceed()
  {
    var agent = new MyAgent();

    var payloadId = Guid.NewGuid()
                        .ToString();
    var taskId = Guid.NewGuid()
                     .ToString();
    var token = Guid.NewGuid()
                    .ToString();
    var sessionId = Guid.NewGuid()
                        .ToString();
    var dd1 = Guid.NewGuid()
                  .ToString();
    var eok1 = Guid.NewGuid()
                   .ToString();

    var folder = Path.Combine(Path.GetTempPath(),
                              token);

    Directory.CreateDirectory(folder);

    var payloadBytes = Encoding.ASCII.GetBytes("payload");
    var dd1Bytes     = Encoding.ASCII.GetBytes("DataDependency1");
    var eok1Bytes    = Encoding.ASCII.GetBytes("ExpectedOutput1");

    await File.WriteAllBytesAsync(Path.Combine(folder,
                                               payloadId),
                                  payloadBytes);
    await File.WriteAllBytesAsync(Path.Combine(folder,
                                               dd1),
                                  dd1Bytes);

    var handler = new TaskHandler(new ProcessRequest
                                  {
                                    CommunicationToken = token,
                                    DataFolder         = folder,
                                    PayloadId          = payloadId,
                                    SessionId          = sessionId,
                                    Configuration = new Configuration
                                                    {
                                                      DataChunkMaxSize = 84,
                                                    },
                                    DataDependencies =
                                    {
                                      dd1,
                                    },
                                    ExpectedOutputKeys =
                                    {
                                      eok1,
                                    },
                                    TaskId = taskId,
                                  },
                                  agent,
                                  new LoggerFactory(),
                                  CancellationToken.None);

    Assert.That(() => handler.SendResult(eok1,
                                         eok1Bytes),
                Throws.InstanceOf<NotImplementedException>());

    Assert.Multiple(() =>
                    {
                      Assert.That(handler.Payload,
                                  Is.EqualTo(payloadBytes));
                      Assert.That(handler.SessionId,
                                  Is.EqualTo(sessionId));
                      Assert.That(handler.TaskId,
                                  Is.EqualTo(taskId));
                      Assert.That(handler.DataDependencies[dd1],
                                  Is.EqualTo(dd1Bytes));
                      Assert.That(File.ReadAllBytes(Path.Combine(folder,
                                                                 eok1)),
                                  Is.EqualTo(eok1Bytes));
                    });
  }
}
