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
using ArmoniK.Utils;

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

  [Obsolete]
  [Test]
  public void TestGetServiceConfiguration()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "GetServiceConfiguration")
                         .WaitSync();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.GetServiceConfiguration(new Empty()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "GetServiceConfiguration")
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestCreateSession()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CreateSession")
                         .WaitSync();
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
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestCancelSession()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CancelSession")
                         .WaitSync();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CancelSession(new Session
                                           {
                                             Id = "session-id",
                                           }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CancelSession")
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestCreateSmallTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CreateSmallTasks")
                         .WaitSync();
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
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestCreateLargeTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CreateLargeTasks")
                         .WaitSync();
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
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestListTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "ListTasks")
                         .WaitSync();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.ListTasks(new TaskFilter()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "ListTasks")
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestListSessions()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "ListSessions")
                         .WaitSync();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.ListSessions(new SessionFilter()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "ListSessions")
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestCountTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CountTasks")
                         .WaitSync();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CountTasks(new TaskFilter()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CountTasks")
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestTryGetResult()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "TryGetResultStream")
                         .WaitSync();
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
                        .WaitSync();
    // Assert.AreEqual(after - before,
    //                 1);
  }

  [Obsolete]
  [Test]
  public void TestTryGetTaskOutput()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "TryGetTaskOutput")
                         .WaitSync();
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
                        .WaitSync();
    // Assert.AreEqual(after - before,
    //                 1);
  }

  [Obsolete]
  [Test]
  public void TestWaitForAvailability()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "WaitForAvailability")
                         .WaitSync();
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
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestWaitForCompletion()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "WaitForCompletion")
                         .WaitSync();
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
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestCancelTasks()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "CancelTasks")
                         .WaitSync();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CancelTasks(new TaskFilter()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "CancelTasks")
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestTaskStatus()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "GetTaskStatus")
                         .WaitSync();
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
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestResultStatus()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "GetResultStatus")
                         .WaitSync();
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
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Obsolete]
  [Test]
  public void TestWatchResults()
  {
    var before = ConfTest.RpcCalled("Submitter",
                                    "WatchResults")
                         .WaitSync();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.WatchResults(),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Submitter",
                                   "WatchResults")
                        .WaitSync();
    // Assert.AreEqual(after - before,
    //                 1);
  }
}
