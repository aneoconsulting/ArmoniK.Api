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

using System.Text;

using ArmoniK.Api.Mock;
using ArmoniK.Api.Mock.Services;

using Microsoft.AspNetCore.Server.Kestrel.Core;

using Newtonsoft.Json;

var builder = WebApplication.CreateBuilder(args);

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
                             options.ListenAnyIP(1080,
                                                 listenOptions =>
                                                 {
                                                   listenOptions.Protocols = HttpProtocols.Http2;
                                                 });
                             options.ListenAnyIP(1081,
                                                 listenOptions => listenOptions.Protocols = HttpProtocols.Http1);
                             options.ListenAnyIP(5001,
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
