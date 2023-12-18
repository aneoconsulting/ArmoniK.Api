// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2023.All rights reserved.
// 
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Threading.Tasks;

using ArmoniK.Api.Mock;
using ArmoniK.Api.Mock.Services;

using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Server.Kestrel.Core;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;

using Newtonsoft.Json;

using Results = ArmoniK.Api.Mock.Services.Results;

var builder = WebApplication.CreateBuilder(args);
builder.Configuration.SetBasePath(Directory.GetCurrentDirectory())
       .AddJsonFile("appsettings.json",
                    true,
                    false)
       .AddEnvironmentVariables()
       .AddCommandLine(args);

var httpPort = int.Parse(builder.Configuration.GetSection("Http")
                                .GetSection("Port")
                                .Value ?? "5000");
var grpcPort = int.Parse(builder.Configuration.GetSection("Grpc")
                                .GetSection("Port")
                                .Value ?? "5001");

// Additional configuration is required to successfully run gRPC on macOS.
// For instructions on how to configure Kestrel and gRPC clients on macOS, visit https://go.microsoft.com/fwlink/?linkid=2099682

// Add services to the container.
builder.Services.AddGrpc();
foreach (var service in CountingService.GetServices())
{
  builder.Services.AddSingleton(service);
}

builder.WebHost.UseKestrel(options =>
                           {
                             options.ListenAnyIP(httpPort,
                                                 listenOptions => listenOptions.Protocols = HttpProtocols.Http1);
                             options.ListenAnyIP(grpcPort,
                                                 listenOptions => listenOptions.Protocols = HttpProtocols.Http2);
                           });

var app = builder.Build();

app.UseRouting();
app.UseGrpcWeb(new GrpcWebOptions
               {
                 DefaultEnabled = true,
               });

// Configure the HTTP request pipeline.
app.MapGrpcService<Agent>();
app.MapGrpcService<Applications>();
app.MapGrpcService<Authentication>();
app.MapGrpcService<Events>();
app.MapGrpcService<HealthChecks>();
app.MapGrpcService<Partitions>();
app.MapGrpcService<Results>();
app.MapGrpcService<Sessions>();
app.MapGrpcService<Submitter>();
app.MapGrpcService<Tasks>();
app.MapGrpcService<Versions>();
app.MapGet("/",
           ()
             => "Communication with gRPC endpoints must be made through a gRPC client. To learn how to create a client, visit: https://go.microsoft.com/fwlink/?linkid=2086909");
app.MapGet("/calls.json",
           Calls);
app.MapPost("/calls.json",
            Calls);
app.MapPost("/reset",
            _ =>
            {
              CountingService.ResetCounters();
              return Task.CompletedTask;
            });

app.Run();

async Task<byte[]> ReadAll(Stream stream)
{
  await using var ms = new MemoryStream();
  await stream.CopyToAsync(ms);
  return ms.ToArray();
}

async Task Calls(HttpContext context)
{
  var requestBody = Encoding.ASCII.GetString(await ReadAll(context.Request.Body));
  var exclude = string.IsNullOrWhiteSpace(requestBody)
                  ? null
                  : JsonConvert.DeserializeObject<Dictionary<string, HashSet<string>>>(requestBody);
  var body = JsonConvert.SerializeObject(CountingService.GetCounters(exclude));
  context.Response.ContentType = "application/json";
  await context.Response.Body.WriteAsync(Encoding.ASCII.GetBytes(body));
}
