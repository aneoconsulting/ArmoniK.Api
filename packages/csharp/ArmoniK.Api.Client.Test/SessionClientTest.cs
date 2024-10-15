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
  [Test]
  public void TestCreateSession()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestCancelSession()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestGetSession()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestListSessions()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestPauseSession()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestResumeSession()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestPurgeSession()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestDeleteSession()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestStopSubmission()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }

  [Test]
  public void TestCloseSession()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
                                                   {
                                                     Endpoint = endpoint,
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
  }
}
