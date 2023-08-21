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

using System.Text;

using ArmoniK.Api.Mock;
using ArmoniK.Api.Mock.Services;

using Microsoft.AspNetCore.Server.Kestrel.Core;

using Newtonsoft.Json;

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
builder.Services.AddSingleton<AgentService>();
builder.Services.AddSingleton<ApplicationsService>();
builder.Services.AddSingleton<AuthService>();
builder.Services.AddSingleton<EventsService>();
builder.Services.AddSingleton<PartitionsService>();
builder.Services.AddSingleton<ResultsService>();
builder.Services.AddSingleton<SessionsService>();
builder.Services.AddSingleton<SubmitterService>();
builder.Services.AddSingleton<TasksService>();
builder.Services.AddSingleton<VersionsService>();

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
app.MapGrpcService<AgentService>();
app.MapGrpcService<ApplicationsService>();
app.MapGrpcService<AuthService>();
app.MapGrpcService<EventsService>();
app.MapGrpcService<PartitionsService>();
app.MapGrpcService<ResultsService>();
app.MapGrpcService<SessionsService>();
app.MapGrpcService<SubmitterService>();
app.MapGrpcService<TasksService>();
app.MapGrpcService<VersionsService>();
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
