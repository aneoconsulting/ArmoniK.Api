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

using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using System.Threading.Tasks;

using ArmoniK.Api.Common.Channel.Utils;
using ArmoniK.Api.Common.Options;
using ArmoniK.Api.gRPC.V1.Worker;
using ArmoniK.Api.Mock;
using ArmoniK.Api.Mock.Services;

using Google.Protobuf;

using Microsoft.AspNetCore.Authentication.Certificate;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Server.Kestrel.Core;
using Microsoft.AspNetCore.Server.Kestrel.Https;
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

X509Certificate2? serverCert = null;

var serverCertPath = builder.Configuration.GetSection("Http")
                            .GetSection("Cert")
                            .Value ?? "";

var serverKeyPath = builder.Configuration.GetSection("Http")
                           .GetSection("Key")
                           .Value ?? "";
var clientCertPath = builder.Configuration.GetSection("Http")
                            .GetSection("ClientCert")
                            .Value;
if (string.IsNullOrEmpty(clientCertPath))
{
  clientCertPath = null;
}

if (!string.IsNullOrEmpty(serverCertPath))
{
  serverCert = X509Certificate2.CreateFromPemFile(serverCertPath,
                                                  string.IsNullOrEmpty(serverKeyPath)
                                                    ? null
                                                    : serverKeyPath);
  serverCert = new X509Certificate2(serverCert.Export(X509ContentType.Pfx));
}


// Additional configuration is required to successfully run gRPC on macOS.
// For instructions on how to configure Kestrel and gRPC clients on macOS, visit https://go.microsoft.com/fwlink/?linkid=2099682

// Add services to the container.
builder.Services.AddGrpc();
builder.Services.AddLogging();
foreach (var service in CountingService.GetServices())
{
  builder.Services.AddSingleton(service);
}

var rawComputePlaneOptions = builder.Configuration.GetSection(ComputePlane.SettingSection);
var workerChannelOptions   = rawComputePlaneOptions?.GetSection(ComputePlane.WorkerChannelSection);

builder.Services.AddSingleton(_ => new GrpcChannel
                                   {
                                     Address    = workerChannelOptions?.GetValue<string>("Address")            ?? "/cache/armonik_worker.sock",
                                     SocketType = workerChannelOptions?.GetValue<GrpcSocketType>("SocketType") ?? GrpcSocketType.UnixDomainSocket,
                                   });
builder.Services.AddSingleton<GrpcChannelProvider>();
builder.Services.AddSingleton<WorkerCallService>();

if (clientCertPath is not null)
{
  builder.Services.AddAuthentication(CertificateAuthenticationDefaults.AuthenticationScheme)
         .AddCertificate(options =>
                         {
                           options.AllowedCertificateTypes  = CertificateTypes.Chained;
                           options.RevocationMode           = X509RevocationMode.NoCheck;
                           options.ChainTrustValidationMode = X509ChainTrustMode.CustomRootTrust;
                           options.CustomTrustStore.ImportFromPemFile(clientCertPath);
                           options.Events = new CertificateAuthenticationEvents
                                            {
                                              OnAuthenticationFailed = context =>
                                                                       {
                                                                         context.Fail("Auth failure: Bad Client Certificate");

                                                                         return Task.CompletedTask;
                                                                       },
                                            };
                         });

  builder.Services.AddAuthorizationBuilder()
         .SetFallbackPolicy(new AuthorizationPolicyBuilder().RequireAuthenticatedUser()
                                                            .Build());
}

builder.WebHost.UseKestrel(options =>
                           {
                             options.ConfigureHttpsDefaults(configureOptions =>
                                                            {
                                                              configureOptions.ServerCertificate = serverCert;

                                                              if (clientCertPath is not null)
                                                              {
                                                                configureOptions.ClientCertificateMode = ClientCertificateMode.RequireCertificate;
                                                                configureOptions.AllowAnyClientCertificate();
                                                              }
                                                            });

                             options.ListenAnyIP(httpPort,
                                                 listenOptions =>
                                                 {
                                                   listenOptions.Protocols = HttpProtocols.Http1AndHttp2AndHttp3;
                                                   if (serverCert is not null)
                                                   {
                                                     listenOptions.UseHttps();
                                                   }
                                                 });
                             if (grpcPort != httpPort)
                             {
                               options.ListenAnyIP(grpcPort,
                                                   listenOptions =>
                                                   {
                                                     listenOptions.Protocols = HttpProtocols.Http2;
                                                     if (serverCert is not null)
                                                     {
                                                       listenOptions.UseHttps();
                                                     }
                                                   });
                             }
                           });

var app = builder.Build();

app.UseRouting();

if (clientCertPath is not null)
{
  app.UseAuthentication();
  app.UseAuthorization();
}

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
app.MapPost("/worker/process",
            SendProcessRequest);
app.MapPost("/worker/healthcheck",
            SendHealthCheck);

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

async Task SendProcessRequest(HttpContext context)
{
  var requestBody  = Encoding.ASCII.GetString(await ReadAll(context.Request.Body));
  var requestInput = JsonConvert.DeserializeObject<WorkerCallServiceInputModel>(requestBody);
  var request      = new JsonParser(JsonParser.Settings.Default).Parse<ProcessRequest>(requestInput.Request);
  foreach (var result in requestInput.Results)
  {
    await using var file = File.OpenWrite(Path.Join(request.DataFolder,
                                                    result.Key));
    if (requestInput.ResultsEncoding is ResultsEncoding.Base64)
    {
      await file.WriteAsync(Convert.FromBase64String(result.Value));
    }
    else
    {
      await file.WriteAsync(Convert.FromHexString(result.Value));
    }
  }

  var reply = await app.Services.GetRequiredService<WorkerCallService>()
                       .ProcessRequest(request);
  context.Response.ContentType = "application/json";
  await context.Response.Body.WriteAsync(Encoding.ASCII.GetBytes(new JsonFormatter(JsonFormatter.Settings.Default).Format(reply)));
}

async Task SendHealthCheck(HttpContext context)
{
  var reply = await app.Services.GetRequiredService<WorkerCallService>()
                       .HealthCheckRequest();
  context.Response.ContentType = "application/json";
  await context.Response.Body.WriteAsync(Encoding.ASCII.GetBytes(new JsonFormatter(JsonFormatter.Settings.Default).Format(reply)));
}
