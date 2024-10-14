// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2024. All rights reserved.
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
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Results;
using ArmoniK.Utils;

using NUnit.Framework;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class ConnectivityTests
{
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
  public void ResultsGetServiceConfiguration()
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

    var resultClient = new Results.ResultsClient(channel);

    Assert.That(() => resultClient.GetServiceConfiguration(new Empty()),
                Throws.Nothing);
  }

  [Test]
  public async Task MultipleChannels([Values(1,
                                             2,
                                             10,
                                             100)]
                                     int concurrency)
  {
    var channels = await Enumerable.Range(0,
                                          concurrency)
                                   .ParallelSelect(new ParallelTaskOptions(-1),
                                                   i => Task.FromResult(GrpcChannelFactory.CreateChannel(new GrpcClient
                                                                                                         {
                                                                                                           Endpoint              = endpoint_,
                                                                                                           AllowUnsafeConnection = isInsecure_,
                                                                                                           CertPem               = certPath_!,
                                                                                                           KeyPem                = keyPath_!,
                                                                                                           CaCert                = CaCertPath_!,
                                                                                                           HttpMessageHandler    = MessageHandler_!,
                                                                                                         })))
                                   .ToListAsync()
                                   .ConfigureAwait(false);

    await channels.ParallelForEach(async channel =>
                                   {
                                     var resultClient = new Results.ResultsClient(channel);
                                     await resultClient.GetServiceConfigurationAsync(new Empty())
                                                       .ConfigureAwait(false);
                                   })
                  .ConfigureAwait(false);
  }
}
