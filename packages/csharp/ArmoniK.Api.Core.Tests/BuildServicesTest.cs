// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2023. All rights reserved.
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

using System.IO;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1.Applications;

using Grpc.Core;
using Grpc.Net.Client;

using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;

using NUnit.Framework;

namespace ArmoniK.Api.Core.Tests;

[TestFixture]
public class BuildServicesTest
{
  [SetUp]
  public void SetUp()
  {
  }

  [TearDown]
  public virtual void TearDown()
  {
  }

  [Test]
  public async Task BuildServer()
  {
    var builder = WebApplication.CreateBuilder();

    builder.Configuration.SetBasePath(Directory.GetCurrentDirectory())
           .AddJsonFile("appsettings.json",
                        true,
                        false)
           .AddEnvironmentVariables();

    builder.Services.AddLogging()
           .AddGrpcReflection()
           .AddGrpc(options => options.MaxReceiveMessageSize = null)
           .AddJsonTranscoding();

    builder.WebHost.UseTestServer(options => options.PreserveExecutionContext = true);

    var app = builder.Build();

    app.UseRouting();
    app.MapGrpcService<TestService>();

    if (app.Environment.IsDevelopment())
    {
      app.MapGrpcReflectionService();
    }

    await app.StartAsync();

    var server  = app.GetTestServer();
    var handler = server.CreateHandler();
    var channel = GrpcChannel.ForAddress("http://localhost",
                                         new GrpcChannelOptions
                                         {
                                           HttpHandler = handler,
                                         });

    var client = new Applications.ApplicationsClient(channel);

    Assert.ThrowsAsync<RpcException>(async () => await client.CountTasksByStatusAsync(new CountTasksByStatusRequest
                                                                                      {
                                                                                        Name    = "",
                                                                                        Version = "",
                                                                                      }));
    await app.StopAsync();
  }
}

public class TestService : Applications.ApplicationsBase
{
}
