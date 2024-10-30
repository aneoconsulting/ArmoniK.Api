// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-$CURRENT_YEAR$. All rights reserved.
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
using System.Runtime.InteropServices;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Sessions;

using Google.Protobuf.WellKnownTypes;

using NUnit.Framework;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class SessionClientTest
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
  public void TestCreateSession()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "CreateSession")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
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
    var after = ConfTest.RpcCalled("Sessions",
                                   "CreateSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCancelSession()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "CancelSession")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = client.CreateSession(new CreateSessionRequest
                                       {
                                         DefaultTaskOption = taskOptions,
                                         PartitionIds =
                                         {
                                           partition,
                                         },
                                       });

    Assert.That(() => client.CancelSession(new CancelSessionRequest
                                           {
                                             SessionId = session.SessionId,
                                           }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "CancelSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestGetSession()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "GetSession")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = client.CreateSession(new CreateSessionRequest
                                       {
                                         DefaultTaskOption = taskOptions,
                                         PartitionIds =
                                         {
                                           partition,
                                         },
                                       });

    Assert.That(() => client.GetSession(new GetSessionRequest
                                        {
                                          SessionId = session.SessionId,
                                        }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "GetSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestListSessions()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "ListSessions")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };

    var numSessions = 5;

    for (var i = 0; i < numSessions; i++)
    {
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

    Assert.That(() => client.ListSessions(new ListSessionsRequest()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "ListSessions")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestPauseSession()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "PauseSession")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = client.CreateSession(new CreateSessionRequest
                                       {
                                         DefaultTaskOption = taskOptions,
                                         PartitionIds =
                                         {
                                           partition,
                                         },
                                       });

    Assert.That(() => client.PauseSession(new PauseSessionRequest
                                          {
                                            SessionId = session.SessionId,
                                          }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "PauseSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestResumeSession()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "ResumeSession")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = client.CreateSession(new CreateSessionRequest
                                       {
                                         DefaultTaskOption = taskOptions,
                                         PartitionIds =
                                         {
                                           partition,
                                         },
                                       });
    Assert.That(() => client.PauseSession(new PauseSessionRequest
                                          {
                                            SessionId = session.SessionId,
                                          }),
                Throws.Nothing);
    Assert.That(() => client.ResumeSession(new ResumeSessionRequest
                                           {
                                             SessionId = session.SessionId,
                                           }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "ResumeSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestPurgeSession()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "PurgeSession")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = client.CreateSession(new CreateSessionRequest
                                       {
                                         DefaultTaskOption = taskOptions,
                                         PartitionIds =
                                         {
                                           partition,
                                         },
                                       });

    Assert.That(() => client.CloseSession(new CloseSessionRequest
                                          {
                                            SessionId = session.SessionId,
                                          }),
                Throws.Nothing);
    Assert.That(() => client.PurgeSession(new PurgeSessionRequest
                                          {
                                            SessionId = session.SessionId,
                                          }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "PurgeSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestDeleteSession()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "DeleteSession")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = client.CreateSession(new CreateSessionRequest
                                       {
                                         DefaultTaskOption = taskOptions,
                                         PartitionIds =
                                         {
                                           partition,
                                         },
                                       });

    Assert.That(() => client.DeleteSession(new DeleteSessionRequest
                                           {
                                             SessionId = session.SessionId,
                                           }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "DeleteSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestStopSubmission()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "StopSubmission")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = client.CreateSession(new CreateSessionRequest
                                       {
                                         DefaultTaskOption = taskOptions,
                                         PartitionIds =
                                         {
                                           partition,
                                         },
                                       });

    Assert.That(() => client.StopSubmission(new StopSubmissionRequest
                                            {
                                              SessionId = session.SessionId,
                                            }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "StopSubmission")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCloseSession()
  {
    var before = ConfTest.RpcCalled("Sessions",
                                    "CloseSession")
                         .GetAwaiter()
                         .GetResult();
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
    var client    = new Sessions.SessionsClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = client.CreateSession(new CreateSessionRequest
                                       {
                                         DefaultTaskOption = taskOptions,
                                         PartitionIds =
                                         {
                                           partition,
                                         },
                                       });

    Assert.That(() => client.CloseSession(new CloseSessionRequest
                                          {
                                            SessionId = session.SessionId,
                                          }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Sessions",
                                   "CloseSession")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }
}
