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
using System.Text;
using System.Threading;
using System.Runtime.InteropServices;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Results;
using ArmoniK.Api.gRPC.V1.Sessions;

using Google.Protobuf.WellKnownTypes;

using NUnit.Framework;

using FilterField = ArmoniK.Api.gRPC.V1.Results.FilterField;
using Filters = ArmoniK.Api.gRPC.V1.Results.Filters;
using FiltersAnd = ArmoniK.Api.gRPC.V1.Results.FiltersAnd;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class ResultsClientTest
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
  public void TestGetResult()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "GetResult")
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
    var client = new Results.ResultsClient(channel);
    Assert.That(() => client.GetResult(new GetResultRequest
                                       {
                                         ResultId = "result-name",
                                       }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "GetResult")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestGetOwnerTaskId()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "GetOwnerTaskId")
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
    var client    = new Results.ResultsClient(channel);
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
    Assert.That(() => client.GetOwnerTaskId(new GetOwnerTaskIdRequest
                                            {
                                              SessionId = session.SessionId,
                                              ResultId =
                                              {
                                                "result-name",
                                              },
                                            }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "GetOwnerTaskId")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCreateResultMetaData()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "CreateResultsMetaData")
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
    var client    = new Results.ResultsClient(channel);
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
    Assert.That(() => client.CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                   {
                                                     SessionId = session.SessionId,
                                                     Results =
                                                     {
                                                       new CreateResultsMetaDataRequest.Types.ResultCreate
                                                       {
                                                         Name = "result-name",
                                                       },
                                                     },
                                                   }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "CreateResultsMetaData")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestCreateResult()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "CreateResults")
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
    var client    = new Results.ResultsClient(channel);
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
    Assert.That(() => client.CreateResults(new CreateResultsRequest
                                           {
                                             SessionId = session.SessionId,
                                             Results =
                                             {
                                               new CreateResultsRequest.Types.ResultCreate
                                               {
                                                 Name = "result-name",
                                               },
                                             },
                                           }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "CreateResults")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestListResults()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "ListResults")
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
    var client    = new Results.ResultsClient(channel);
    Assert.That(() => client.ListResults(new ListResultsRequest
                                         {
                                           Filters = new Filters
                                                     {
                                                       Or =
                                                       {
                                                         new FiltersAnd
                                                         {
                                                           And =
                                                           {
                                                             new FilterField
                                                             {
                                                               Field = new ResultField
                                                                       {
                                                                         ResultRawField = new ResultRawField
                                                                                          {
                                                                                            Field = ResultRawEnumField.ResultId,
                                                                                          },
                                                                       },
                                                               FilterString = new FilterString
                                                                              {
                                                                                Operator = FilterStringOperator.Equal,
                                                                                Value    = "result-id",
                                                                              },
                                                             },
                                                           },
                                                         },
                                                       },
                                                     },
                                         }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "ListResults")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestUploadResults()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "UploadResultData")
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
    var client    = new Results.ResultsClient(channel);
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

    Assert.That(() => client.UploadResultData(session.SessionId,
                                              "result-id",
                                              Encoding.ASCII.GetBytes("result data")),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "UploadResultData")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestDownloadResults()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "DownloadResultData")
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
    var client    = new Results.ResultsClient(channel);
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
    Assert.That(() => client.DownloadResultData(session.SessionId,
                                                "result-id",
                                                CancellationToken.None),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "DownloadResultData")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestDeleteResults()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "DeleteResultsData")
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
    var client    = new Results.ResultsClient(channel);
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
    Assert.That(() => client.DeleteResultsData(new DeleteResultsDataRequest
                                               {
                                                 SessionId = session.SessionId,
                                                 ResultId =
                                                 {
                                                   "result-id",
                                                 },
                                               }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "DeleteResultsData")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestWatchResults()
  {
    var before = ConfTest.RpcCalled("Results",
                                    "WatchResults")
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
    var client = new Results.ResultsClient(channel);
    Assert.That(() => client.WatchResults(),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Results",
                                   "WatchResults")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    0);
  }
}
