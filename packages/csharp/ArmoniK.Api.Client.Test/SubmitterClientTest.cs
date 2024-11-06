// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-$CURRENT_YEAR.All rights reserved.
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

using System;
using System.Linq;
using System.Threading;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Submitter;

using Google.Protobuf.WellKnownTypes;

using NUnit.Framework;

using Empty = ArmoniK.Api.gRPC.V1.Empty;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class SubmitterClientTest
{
  [SetUp]
  public void SetUp()
    => options_ = ConfTest.GetChannelOptions();

  private GrpcClient? options_;

  [Test]
  public void TestGetServiceConfiguration()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "GetServiceConfiguration")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.GetServiceConfiguration(new Empty()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "GetServiceConfiguration")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCreateSession()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CreateSession")
                         .GetAwaiter()
                         .GetResult();
    var channel   = GrpcChannelFactory.CreateChannel(options_!);
    var partition = "default";
    var client    = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    Assert.That(() => client.CreateSession(new CreateSessionRequest
                                           {
                                             DefaultTaskOption = taskOptions,
                                             PartitionIds =
                                             {
                                               partition,
                                             },
                                           }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CreateSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCancelSession()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CancelSession")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CancelSession(new Session
                                           {
                                             Id = "session-id",
                                           }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CancelSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCreateSmallTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CreateSmallTasks")
                         .GetAwaiter()
                         .GetResult();
    var channel   = GrpcChannelFactory.CreateChannel(options_!);
    var partition = "default";
    var client    = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    Assert.That(() => client.CreateSmallTasks(new CreateSmallTaskRequest
                                              {
                                                SessionId   = "session-id",
                                                TaskOptions = taskOptions,
                                              }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CreateSmallTasks")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCreateLargeTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CreateLargeTasks")
                         .GetAwaiter()
                         .GetResult();
    var channel   = GrpcChannelFactory.CreateChannel(options_!);
    var partition = "default";
    var client    = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    Assert.That(() => client.CreateTasksAsync("session-id",
                                              taskOptions,
                                              Enumerable.Empty<TaskRequest>(),
                                              CancellationToken.None),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CreateLargeTasks")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestListTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "ListTasks")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.ListTasks(new TaskFilter()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "ListTasks")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestListSessions()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "ListSessions")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.ListSessions(new SessionFilter()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "ListSessions")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCountTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CountTasks")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CountTasks(new TaskFilter()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CountTasks")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestTryGetResult()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "TryGetResultStream")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.TryGetResultStream(new ResultRequest
                                                {
                                                  ResultId = "result-id",
                                                  Session  = "session-id",
                                                }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "TryGetResultStream")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    0);
  }

  [Test]
  public void TestTryGetTaskOutput()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "TryGetTaskOutput")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.TryGetTaskOutput(new TaskOutputRequest
                                              {
                                                Session = "session-id",
                                                TaskId  = "task-id",
                                              }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "TryGetTaskOutput")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestWaitForAvailability()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "WaitForAvailability")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.WaitForAvailability(new ResultRequest
                                                 {
                                                   ResultId = "result-id",
                                                   Session  = "session-id",
                                                 }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "WaitForAvailability")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestWaitForCompletion()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "WaitForCompletion")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.WaitForCompletion(new WaitRequest
                                               {
                                                 Filter                      = new TaskFilter(),
                                                 StopOnFirstTaskError        = true,
                                                 StopOnFirstTaskCancellation = true,
                                               }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "WaitForCompletion")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCancelTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CancelTasks")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CancelTasks(new TaskFilter()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CancelTasks")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestTaskStatus()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "GetTaskStatus")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.GetTaskStatus(new GetTaskStatusRequest
                                           {
                                             TaskIds =
                                             {
                                               "task-id",
                                             },
                                           }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "GetTaskStatus")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestResultStatus()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "GetResultStatus")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.GetResultStatus(new GetResultStatusRequest
                                             {
                                               SessionId = "session-id",
                                               ResultIds =
                                               {
                                                 "result-id",
                                               },
                                             }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "GetResultStatus")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestWatchResults()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "WatchResults")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.WatchResults(),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "WatchResults")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    0);
  }
}
