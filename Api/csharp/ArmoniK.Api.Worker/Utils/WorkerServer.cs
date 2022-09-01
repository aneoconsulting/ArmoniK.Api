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
  {
    try
    {
      var builder = WebApplication.CreateBuilder();

      builder.Configuration.SetBasePath(Directory.GetCurrentDirectory())
             .AddJsonFile("appsettings.json",
                          true,
                          false)
             .AddEnvironmentVariables();

      if (configuration is not null)
      {
        builder.Configuration.AddConfiguration(configuration);
      }

      Log.Logger = new LoggerConfiguration().ReadFrom.Configuration(builder.Configuration)
                                            .WriteTo.Console(new CompactJsonFormatter())
                                            .Enrich.FromLogContext()
                                            .CreateLogger();

      var loggerFactory = LoggerFactory.Create(loggingBuilder => loggingBuilder.AddSerilog(Log.Logger));
      var logger        = loggerFactory.CreateLogger("root");

      builder.Host.UseSerilog(Log.Logger);


      var computePlanOptions = builder.Configuration.GetRequiredSection(ComputePlane.SettingSection)
                                      .Get<ComputePlane>();

      if (computePlanOptions.WorkerChannel == null)
      {
        throw new Exception($"{nameof(computePlanOptions.WorkerChannel)} options should not be null");
      }

      builder.WebHost.ConfigureKestrel(options => options.ListenUnixSocket(computePlanOptions.WorkerChannel.Address,
                                                                           listenOptions =>
                                                                           {
                                                                             if (File.Exists(computePlanOptions.WorkerChannel.Address))
                                                                             {
                                                                               File.Delete(computePlanOptions.WorkerChannel.Address);
                                                                             }

                                                                             listenOptions.Protocols = HttpProtocols.Http2;
                                                                           }));

      builder.Services.AddSingleton<ApplicationLifeTimeManager>()
             .AddSingleton(_ => loggerFactory)
             .AddSingleton<GrpcChannelProvider>()
             .AddSingleton(computePlanOptions.AgentChannel)
             .AddLogging()
             .AddGrpcReflection()
             .AddGrpc(options => options.MaxReceiveMessageSize = null);

      serviceConfigurator?.Invoke(builder.Services);

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
