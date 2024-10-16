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
using System.Linq;
using System.Text;
using System.Runtime.InteropServices;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Results;
using ArmoniK.Api.gRPC.V1.Sessions;
using ArmoniK.Api.gRPC.V1.Tasks;

using Google.Protobuf;
using Google.Protobuf.WellKnownTypes;

using NUnit.Framework;

using FilterField = ArmoniK.Api.gRPC.V1.Tasks.FilterField;
using Filters = ArmoniK.Api.gRPC.V1.Tasks.Filters;
using FiltersAnd = ArmoniK.Api.gRPC.V1.Tasks.FiltersAnd;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class TasksClientTest
{
  private static FiltersAnd TasksFilter(string sessionId)
    => new()
       {
         And =
         {
           new FilterField
           {
             Field = new TaskField
                     {
                       TaskSummaryField = new TaskSummaryField
                                          {
                                            Field = TaskSummaryEnumField.SessionId,
                                          },
                     },
             FilterString = new FilterString
                            {
                              Operator = FilterStringOperator.Equal,
                              Value    = sessionId,
                            },
           },
         },
       };

  [SetUp]
  public void SetUp()
  {
    certPath_       = Environment.GetEnvironmentVariable("Grpc__ClientCert")               ?? "";
    keyPath_        = Environment.GetEnvironmentVariable("Grpc__ClientKey")                ?? "";
    CaCertPath_     = Environment.GetEnvironmentVariable("Grpc__CaCert")                   ?? "";
    MessageHandler_ = Environment.GetEnvironmentVariable("GrpcClient__HttpMessageHandler") ?? "";
    endpoint_       = Environment.GetEnvironmentVariable("Grpc__Endpoint")                 ?? "";
    isInsecure_     = IsInsecure(endpoint_);

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

  private static bool IsInsecure(string endpoint)
  {
    var uri = new Uri(endpoint);
    return uri.Scheme == Uri.UriSchemeHttp;
  }

  [Test]
  public void TestSubmitTask()
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
    var client    = new Tasks.TasksClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = new Sessions.SessionsClient(channel).CreateSession(new CreateSessionRequest
                                                                     {
                                                                       DefaultTaskOption = taskOptions,
                                                                       PartitionIds =
                                                                       {
                                                                         partition,
                                                                       },
                                                                     });
    var resultId = new Results.ResultsClient(channel).CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                                            {
                                                                              SessionId = session.SessionId,
                                                                              Results =
                                                                              {
                                                                                new CreateResultsMetaDataRequest.Types.ResultCreate
                                                                                {
                                                                                  Name = "Result",
                                                                                },
                                                                              },
                                                                            })
                                                     .Results.Single()
                                                     .ResultId;
    var payloadId = new Results.ResultsClient(channel).CreateResults(new CreateResultsRequest
                                                                     {
                                                                       SessionId = session.SessionId,
                                                                       Results =
                                                                       {
                                                                         new CreateResultsRequest.Types.ResultCreate
                                                                         {
                                                                           Data = UnsafeByteOperations.UnsafeWrap(Encoding.ASCII.GetBytes("Hello")),
                                                                           Name = "Payload",
                                                                         },
                                                                       },
                                                                     })
                                                      .Results.Single()
                                                      .ResultId;
    Assert.That(() => client.SubmitTasks(new SubmitTasksRequest
                                         {
                                           SessionId = session.SessionId,
                                           TaskCreations =
                                           {
                                             new SubmitTasksRequest.Types.TaskCreation
                                             {
                                               PayloadId = payloadId,
                                               ExpectedOutputKeys =
                                               {
                                                 resultId,
                                               },
                                             },
                                           },
                                         }),
                Throws.Nothing);
  }

  [Test]
  public void TestCountTask()
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
    var client    = new Tasks.TasksClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = new Sessions.SessionsClient(channel).CreateSession(new CreateSessionRequest
                                                                     {
                                                                       DefaultTaskOption = taskOptions,
                                                                       PartitionIds =
                                                                       {
                                                                         partition,
                                                                       },
                                                                     });
    var resultId = new Results.ResultsClient(channel).CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                                            {
                                                                              SessionId = session.SessionId,
                                                                              Results =
                                                                              {
                                                                                new CreateResultsMetaDataRequest.Types.ResultCreate
                                                                                {
                                                                                  Name = "Result",
                                                                                },
                                                                              },
                                                                            })
                                                     .Results.Single()
                                                     .ResultId;
    var payloadId = new Results.ResultsClient(channel).CreateResults(new CreateResultsRequest
                                                                     {
                                                                       SessionId = session.SessionId,
                                                                       Results =
                                                                       {
                                                                         new CreateResultsRequest.Types.ResultCreate
                                                                         {
                                                                           Data = UnsafeByteOperations.UnsafeWrap(Encoding.ASCII.GetBytes("Hello")),
                                                                           Name = "Payload",
                                                                         },
                                                                       },
                                                                     })
                                                      .Results.Single()
                                                      .ResultId;
    var submitResponse = client.SubmitTasks(new SubmitTasksRequest
                                            {
                                              SessionId = session.SessionId,
                                              TaskCreations =
                                              {
                                                new SubmitTasksRequest.Types.TaskCreation
                                                {
                                                  PayloadId = payloadId,
                                                  ExpectedOutputKeys =
                                                  {
                                                    resultId,
                                                  },
                                                },
                                              },
                                            });
    Assert.That(() => client.CountTasksByStatus(new CountTasksByStatusRequest
                                                {
                                                  Filters = new Filters
                                                            {
                                                              Or =
                                                              {
                                                                TasksFilter(session.SessionId),
                                                              },
                                                            },
                                                }),
                Throws.Nothing);
  }

  [Test]
  public void TestGetResultsIds()
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
    var client    = new Tasks.TasksClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = new Sessions.SessionsClient(channel).CreateSession(new CreateSessionRequest
                                                                     {
                                                                       DefaultTaskOption = taskOptions,
                                                                       PartitionIds =
                                                                       {
                                                                         partition,
                                                                       },
                                                                     });
    var resultId = new Results.ResultsClient(channel).CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                                            {
                                                                              SessionId = session.SessionId,
                                                                              Results =
                                                                              {
                                                                                new CreateResultsMetaDataRequest.Types.ResultCreate
                                                                                {
                                                                                  Name = "Result",
                                                                                },
                                                                              },
                                                                            })
                                                     .Results.Single()
                                                     .ResultId;
    var payloadId = new Results.ResultsClient(channel).CreateResults(new CreateResultsRequest
                                                                     {
                                                                       SessionId = session.SessionId,
                                                                       Results =
                                                                       {
                                                                         new CreateResultsRequest.Types.ResultCreate
                                                                         {
                                                                           Data = UnsafeByteOperations.UnsafeWrap(Encoding.ASCII.GetBytes("TestPayload")),
                                                                           Name = "Payload",
                                                                         },
                                                                       },
                                                                     })
                                                      .Results.Single()
                                                      .ResultId;
    var taskId = client.SubmitTasks(new SubmitTasksRequest
                                    {
                                      SessionId = session.SessionId,
                                      TaskCreations =
                                      {
                                        new SubmitTasksRequest.Types.TaskCreation
                                        {
                                          PayloadId = payloadId,
                                          ExpectedOutputKeys =
                                          {
                                            resultId,
                                          },
                                        },
                                      },
                                    })
                       .TaskInfos.Single()
                       .TaskId;
    Assert.That(() => client.GetResultIds(new GetResultIdsRequest
                                          {
                                            TaskId =
                                            {
                                              taskId,
                                            },
                                          }),
                Throws.Nothing);
  }

  [Test]
  public void TestGetTask()
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
    var client    = new Tasks.TasksClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = new Sessions.SessionsClient(channel).CreateSession(new CreateSessionRequest
                                                                     {
                                                                       DefaultTaskOption = taskOptions,
                                                                       PartitionIds =
                                                                       {
                                                                         partition,
                                                                       },
                                                                     });
    var resultId = new Results.ResultsClient(channel).CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                                            {
                                                                              SessionId = session.SessionId,
                                                                              Results =
                                                                              {
                                                                                new CreateResultsMetaDataRequest.Types.ResultCreate
                                                                                {
                                                                                  Name = "Result",
                                                                                },
                                                                              },
                                                                            })
                                                     .Results.Single()
                                                     .ResultId;
    var payloadId = new Results.ResultsClient(channel).CreateResults(new CreateResultsRequest
                                                                     {
                                                                       SessionId = session.SessionId,
                                                                       Results =
                                                                       {
                                                                         new CreateResultsRequest.Types.ResultCreate
                                                                         {
                                                                           Data = UnsafeByteOperations.UnsafeWrap(Encoding.ASCII.GetBytes("TestPayload")),
                                                                           Name = "Payload",
                                                                         },
                                                                       },
                                                                     })
                                                      .Results.Single()
                                                      .ResultId;
    var taskId = client.SubmitTasks(new SubmitTasksRequest
                                    {
                                      SessionId = session.SessionId,
                                      TaskCreations =
                                      {
                                        new SubmitTasksRequest.Types.TaskCreation
                                        {
                                          PayloadId = payloadId,
                                          ExpectedOutputKeys =
                                          {
                                            resultId,
                                          },
                                        },
                                      },
                                    })
                       .TaskInfos.Single()
                       .TaskId;
    Assert.That(() => client.GetTask(new GetTaskRequest
                                     {
                                       TaskId = taskId,
                                     }),
                Throws.Nothing);
  }

  [Test]
  public void TestCancelTask()
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
    var client    = new Tasks.TasksClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = new Sessions.SessionsClient(channel).CreateSession(new CreateSessionRequest
                                                                     {
                                                                       DefaultTaskOption = taskOptions,
                                                                       PartitionIds =
                                                                       {
                                                                         partition,
                                                                       },
                                                                     });
    var resultId = new Results.ResultsClient(channel).CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                                            {
                                                                              SessionId = session.SessionId,
                                                                              Results =
                                                                              {
                                                                                new CreateResultsMetaDataRequest.Types.ResultCreate
                                                                                {
                                                                                  Name = "Result",
                                                                                },
                                                                              },
                                                                            })
                                                     .Results.Single()
                                                     .ResultId;
    var payloadId = new Results.ResultsClient(channel).CreateResults(new CreateResultsRequest
                                                                     {
                                                                       SessionId = session.SessionId,
                                                                       Results =
                                                                       {
                                                                         new CreateResultsRequest.Types.ResultCreate
                                                                         {
                                                                           Data = UnsafeByteOperations.UnsafeWrap(Encoding.ASCII.GetBytes("TestPayload")),
                                                                           Name = "Payload",
                                                                         },
                                                                       },
                                                                     })
                                                      .Results.Single()
                                                      .ResultId;
    var taskId = client.SubmitTasks(new SubmitTasksRequest
                                    {
                                      SessionId = session.SessionId,
                                      TaskCreations =
                                      {
                                        new SubmitTasksRequest.Types.TaskCreation
                                        {
                                          PayloadId = payloadId,
                                          ExpectedOutputKeys =
                                          {
                                            resultId,
                                          },
                                        },
                                      },
                                    })
                       .TaskInfos.Single()
                       .TaskId;
    Assert.AreNotEqual(client.GetTask(new GetTaskRequest
                                      {
                                        TaskId = taskId,
                                      })
                             .Task.Status,
                       TaskStatus.Cancelled);
    Assert.That(() => client.CancelTasks(new CancelTasksRequest
                                         {
                                           TaskIds =
                                           {
                                             taskId,
                                           },
                                         }),
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
    var partition = "default";
    var client    = new Tasks.TasksClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = new Sessions.SessionsClient(channel).CreateSession(new CreateSessionRequest
                                                                     {
                                                                       DefaultTaskOption = taskOptions,
                                                                       PartitionIds =
                                                                       {
                                                                         partition,
                                                                       },
                                                                     });
    var resultId = new Results.ResultsClient(channel).CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                                            {
                                                                              SessionId = session.SessionId,
                                                                              Results =
                                                                              {
                                                                                new CreateResultsMetaDataRequest.Types.ResultCreate
                                                                                {
                                                                                  Name = "Result",
                                                                                },
                                                                              },
                                                                            })
                                                     .Results.Single()
                                                     .ResultId;
    var payloadId = new Results.ResultsClient(channel).CreateResults(new CreateResultsRequest
                                                                     {
                                                                       SessionId = session.SessionId,
                                                                       Results =
                                                                       {
                                                                         new CreateResultsRequest.Types.ResultCreate
                                                                         {
                                                                           Data = UnsafeByteOperations.UnsafeWrap(Encoding.ASCII.GetBytes("TestPayload")),
                                                                           Name = "Payload",
                                                                         },
                                                                       },
                                                                     })
                                                      .Results.Single()
                                                      .ResultId;
    var taskId = client.SubmitTasks(new SubmitTasksRequest
                                    {
                                      SessionId = session.SessionId,
                                      TaskCreations =
                                      {
                                        new SubmitTasksRequest.Types.TaskCreation
                                        {
                                          PayloadId = payloadId,
                                          ExpectedOutputKeys =
                                          {
                                            resultId,
                                          },
                                        },
                                      },
                                    })
                       .TaskInfos.Single()
                       .TaskId;
    Assert.That(() => client.ListTasks(new ListTasksRequest
                                       {
                                         Filters = new Filters
                                                   {
                                                     Or =
                                                     {
                                                       TasksFilter(session.SessionId),
                                                     },
                                                   },
                                       }),
                Throws.Nothing);
  }

  [Test]
  public void TestListTaskDetailed()
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
    var client    = new Tasks.TasksClient(channel);
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var session = new Sessions.SessionsClient(channel).CreateSession(new CreateSessionRequest
                                                                     {
                                                                       DefaultTaskOption = taskOptions,
                                                                       PartitionIds =
                                                                       {
                                                                         partition,
                                                                       },
                                                                     });
    var resultId = new Results.ResultsClient(channel).CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                                            {
                                                                              SessionId = session.SessionId,
                                                                              Results =
                                                                              {
                                                                                new CreateResultsMetaDataRequest.Types.ResultCreate
                                                                                {
                                                                                  Name = "Result",
                                                                                },
                                                                              },
                                                                            })
                                                     .Results.Single()
                                                     .ResultId;
    var payloadId = new Results.ResultsClient(channel).CreateResults(new CreateResultsRequest
                                                                     {
                                                                       SessionId = session.SessionId,
                                                                       Results =
                                                                       {
                                                                         new CreateResultsRequest.Types.ResultCreate
                                                                         {
                                                                           Data = UnsafeByteOperations.UnsafeWrap(Encoding.ASCII.GetBytes("TestPayload")),
                                                                           Name = "Payload",
                                                                         },
                                                                       },
                                                                     })
                                                      .Results.Single()
                                                      .ResultId;
    var taskId = client.SubmitTasks(new SubmitTasksRequest
                                    {
                                      SessionId = session.SessionId,
                                      TaskCreations =
                                      {
                                        new SubmitTasksRequest.Types.TaskCreation
                                        {
                                          PayloadId = payloadId,
                                          ExpectedOutputKeys =
                                          {
                                            resultId,
                                          },
                                        },
                                      },
                                    })
                       .TaskInfos.Single()
                       .TaskId;
    Assert.That(() => client.ListTasksDetailed(new ListTasksRequest
                                               {
                                                 Filters = new Filters
                                                           {
                                                             Or =
                                                             {
                                                               TasksFilter(session.SessionId),
                                                             },
                                                           },
                                               }),
                Throws.Nothing);
  }
}
