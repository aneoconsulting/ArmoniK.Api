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
using System.Runtime.InteropServices;
using System.Threading;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1.Submitter;
using ArmoniK.Api.gRPC.V1;

using Google.Protobuf.WellKnownTypes;

using NUnit.Framework;

using Empty = ArmoniK.Api.gRPC.V1.Empty;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class SubmitterClientTest
{
  [SetUp]
  public void SetUp()
  {
    certPath_       = Environment.GetEnvironmentVariable("Grpc__ClientCert")               ?? "";
    keyPath_        = Environment.GetEnvironmentVariable("Grpc__ClientKey")                ?? "";
    CaCertPath_     = Environment.GetEnvironmentVariable("Grpc__CaCert")                   ?? "";
    MessageHandler_ = Environment.GetEnvironmentVariable("GrpcClient__HttpMessageHandler") ?? "";
    endpoint_       = Environment.GetEnvironmentVariable("Grpc__Endpoint")                 ?? "";
    isInsecure_     = Environment.GetEnvironmentVariable("Grpc__AllowUnsafeConnection") == "true";

    if (isInsecure_)
    {
      endpoint_ = RuntimeInformation.FrameworkDescription.StartsWith(".NET Framework") || MessageHandler_.ToLower()
                                                                                                         .Contains("web")
                    ? "http://localhost:4999"
                    : endpoint_;
    }
  }

  private static string? endpoint_;
  private static string? certPath_;
  private static string? keyPath_;
  private static string? CaCertPath_;
  private static string? MessageHandler_;
  private        bool    isInsecure_;

  [Test]
  public void TestGetServiceConfiguration()
  {
    var val = ConfTest.RpcCalled("",
                                 "");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.GetServiceConfiguration(new Empty()),
                Throws.Nothing);
  }

  [Test]
  public void TestCreateSession()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
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
  }

  [Test]
  public void TestCancelSession()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CancelSession(new Session
                                           {
                                             Id = "session-id",
                                           }),
                Throws.Nothing);
  }

  [Test]
  public void TestCreateSmallTasks()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
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
                                                TaskRequests =
                                                {
                                                },
                                              }),
                Throws.Nothing);
  }

  [Test]
  public void TestCreateLargeTasks()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var partition = "default";
    var client    = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    Assert.That(() => SubmitterClientExt.CreateTasksAsync(client,
                                                          "session-id",
                                                          taskOptions,
                                                          Enumerable.Empty<TaskRequest>(),
                                                          CancellationToken.None),
                Throws.Nothing);
  }

  [Test]
  public void TestListTasks()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.ListTasks(new TaskFilter
                                       {
                                       }),
                Throws.Nothing);
  }

  [Test]
  public void TestListSessions()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.ListSessions(new SessionFilter
                                          {
                                          }),
                Throws.Nothing);
  }

  [Test]
  public void TestCountTasks()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CountTasks(new TaskFilter
                                        {
                                        }),
                Throws.Nothing);
  }

  [Test]
  public void TestTryGetResult()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.TryGetResultStream(new ResultRequest
                                                {
                                                  ResultId = "result-id",
                                                  Session  = "session-id",
                                                }),
                Throws.Nothing);
  }

  [Test]
  public void TestTryGetTaskOutput()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.TryGetTaskOutput(new TaskOutputRequest
                                              {
                                                Session = "session-id",
                                                TaskId  = "task-id",
                                              }),
                Throws.Nothing);
  }

  [Test]
  public void TestWaitForAvailability()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.WaitForAvailability(new ResultRequest
                                                 {
                                                   ResultId = "result-id",
                                                   Session  = "session-id",
                                                 }),
                Throws.Nothing);
  }

  [Test]
  public void TestWaitForCompletion()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.WaitForCompletion(new WaitRequest
                                               {
                                                 Filter = new TaskFilter
                                                          {
                                                          },
                                                 StopOnFirstTaskError        = true,
                                                 StopOnFirstTaskCancellation = true,
                                               }),
                Throws.Nothing);
  }

  [Test]
  public void TestCancelTasks()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.CancelTasks(new TaskFilter
                                         {
                                         }),
                Throws.Nothing);
  }

  [Test]
  public void TestTaskStatus()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.GetTaskStatus(new GetTaskStatusRequest
                                           {
                                             TaskIds =
                                             {
                                               "task-id",
                                             },
                                           }),
                Throws.Nothing);
  }

  [Test]
  public void TestResultStatus()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.GetResultStatus(new GetResultStatusRequest
                                             {
                                               SessionId = "session-id",
                                               ResultIds =
                                               {
                                                 "result-id",
                                               },
                                             }),
                Throws.Nothing);
  }

  [Test]
  public void TestWatchResults()
  {
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint              = endpoint_,
                                                     AllowUnsafeConnection = isInsecure_,
                                                     CertPem               = certPath_!,
                                                     KeyPem                = keyPath_!,
                                                     CaCert                = CaCertPath_!,
                                                     HttpMessageHandler    = MessageHandler_!,
                                                   });
    var client = new gRPC.V1.Submitter.Submitter.SubmitterClient(channel);
    Assert.That(() => client.WatchResults(),
                Throws.Nothing);
  }
}
