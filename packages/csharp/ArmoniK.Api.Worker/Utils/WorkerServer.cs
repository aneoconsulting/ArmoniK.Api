// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2022. All rights reserved.
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
using System.IO;

using ArmoniK.Api.Common.Channel.Utils;
using ArmoniK.Api.Common.Options;
using ArmoniK.Api.Common.Utils;

using JetBrains.Annotations;

using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Server.Kestrel.Core;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

using Serilog;
using Serilog.Formatting.Compact;

namespace ArmoniK.Api.Worker.Utils;

/// <summary>
///   Convenience class to create a web application with the given gRPC Service for ArmoniK Worker
/// </summary>
[PublicAPI]
public static class WorkerServer
{
  /// <summary>
  ///   Create a web application for the given ArmoniK Worker gRPC Service
  /// </summary>
  /// <typeparam name="T">gRPC Service to add to the web application</typeparam>
  /// <param name="configuration">Additional configurations</param>
  /// <param name="serviceConfigurator">Lambda to configure server services</param>
  /// <returns>
  ///   The web application initialized
  /// </returns>
  public static WebApplication Create<T>(IConfiguration?             configuration       = null,
                                         Action<IServiceCollection>? serviceConfigurator = null)
    where T : gRPC.V1.Worker.Worker.WorkerBase
    => Create<T>((collection,
                  configuration1) =>
                 {
                   if (configuration != null)
                   {
                     foreach (var pair in configuration.AsEnumerable())
                     {
                       configuration1[pair.Key] = pair.Value;
                     }
                   }

                   serviceConfigurator?.Invoke(collection);
                 });


  /// <summary>
  ///   Create a web application for the given ArmoniK Worker gRPC Service
  /// </summary>
  /// <typeparam name="T">gRPC Service to add to the web application</typeparam>
  /// <param name="configurator">Lambda to configure server services</param>
  /// <returns>
  ///   The web application initialized
  /// </returns>
  public static WebApplication Create<T>(Action<IServiceCollection, IConfiguration>? configurator)
    where T : gRPC.V1.Worker.Worker.WorkerBase
  {
    try
    {
      var builder = WebApplication.CreateBuilder();

      builder.Configuration.SetBasePath(Directory.GetCurrentDirectory())
             .AddJsonFile("appsettings.json",
                          true,
                          false)
             .AddEnvironmentVariables();

      configurator?.Invoke(builder.Services,
                           builder.Configuration);

      Log.Logger = new LoggerConfiguration().ReadFrom.Configuration(builder.Configuration)
                                            .WriteTo.Console(new CompactJsonFormatter())
                                            .Enrich.FromLogContext()
                                            .CreateLogger();

      var loggerFactory = LoggerFactory.Create(loggingBuilder => loggingBuilder.AddSerilog(Log.Logger));
      var logger        = loggerFactory.CreateLogger("root");

      builder.Host.UseSerilog(Log.Logger);

      var rawComputePlaneOptions = builder.Configuration.GetRequiredSection(ComputePlane.SettingSection);
      var workerChannelOptions = rawComputePlaneOptions.
                                         GetRequiredSection(ComputePlane.WorkerChannelSection);
      var agentChannelOptions = rawComputePlaneOptions.GetRequiredSection(ComputePlane.AgentChannelSection);
      var parsedComputePlaneOptions = new ComputePlane
                                      {
                                        WorkerChannel = new GrpcChannel
                                                        {
                                                          Address = workerChannelOptions.GetRequiredValue<string>("Address"),
                                                          SocketType = workerChannelOptions.GetValue("SocketType", GrpcSocketType.UnixDomainSocket),
                                                          KeepAlivePingTimeOut = workerChannelOptions.GetTimeSpanOrDefault("KeepAlivePingTimeOut",
                                                                                                                           TimeSpan.FromSeconds(20)),
                                                          KeepAliveTimeOut = workerChannelOptions.GetTimeSpanOrDefault("KeepAliveTimeOut",
                                                                                                                       TimeSpan.FromSeconds(130)),
                                                        },
                                        AgentChannel = new GrpcChannel
                                                       {
                                                          Address    = agentChannelOptions.GetRequiredValue<string>("Address"),
                                                          SocketType = agentChannelOptions.GetValue("SocketType", GrpcSocketType.UnixDomainSocket),
                                                          KeepAlivePingTimeOut = agentChannelOptions.GetTimeSpanOrDefault("KeepAlivePingTimeOut",
                                                                                                                           TimeSpan.FromSeconds(20)),
                                                          KeepAliveTimeOut = agentChannelOptions.GetTimeSpanOrDefault("KeepAliveTimeOut",
                                                                                                                      TimeSpan.FromSeconds(130)),
                                                       },
                                        MessageBatchSize = rawComputePlaneOptions.GetValue("MessageBatchSize", 1),
                                        AbortAfter = rawComputePlaneOptions.GetValue("AbortAfter", TimeSpan.Zero),
                                      };

      builder.WebHost.ConfigureKestrel(options =>
                                       {
                                         var address       = parsedComputePlaneOptions.WorkerChannel.Address;
                                         switch (parsedComputePlaneOptions.WorkerChannel.SocketType)
                                         {
                                           case GrpcSocketType.UnixDomainSocket:
                                             if (File.Exists(address))
                                             {
                                               File.Delete(address);
                                             }

                                             options.ListenUnixSocket(address,
                                                                      listenOptions => listenOptions.Protocols = HttpProtocols.Http2);
                                             break;
                                           case GrpcSocketType.Tcp:
                                             options.Limits.KeepAliveTimeout           = parsedComputePlaneOptions.WorkerChannel.KeepAliveTimeOut;
                                             options.Limits.Http2.KeepAlivePingTimeout = parsedComputePlaneOptions.WorkerChannel.KeepAlivePingTimeOut;
                                             var uri = new Uri(address);
                                             options.ListenAnyIP(uri.Port,
                                                                 listenOptions => listenOptions.Protocols = HttpProtocols.Http2);
                                             break;
                                           default:
                                             throw new InvalidOperationException("Socket type unknown");
                                         }
                                       });

      builder.Services.AddSingleton<ApplicationLifeTimeManager>()
             .AddSingleton(_ => loggerFactory)
             .AddSingleton<GrpcChannelProvider>()
             .AddSingleton(parsedComputePlaneOptions)
             .AddSingleton(parsedComputePlaneOptions.AgentChannel)
             .AddLogging()
             .AddGrpcReflection()
             .AddGrpc(options => options.MaxReceiveMessageSize = null);

      var app = builder.Build();

      if (app.Environment.IsDevelopment())
      {
        app.UseDeveloperExceptionPage();
      }

      app.UseSerilogRequestLogging();

      app.UseRouting();

      app.UseEndpoints(endpoints =>
                       {
                         endpoints.MapGrpcService<T>();

                         if (app.Environment.IsDevelopment())
                         {
                           endpoints.MapGrpcReflectionService();
                           logger.LogInformation("Grpc Reflection Activated");
                         }
                       });

      return app;
    }
    catch (Exception ex)
    {
      Log.Fatal(ex,
                "Host terminated unexpectedly");
      throw;
    }
  }
}
