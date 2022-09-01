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
using System.Threading.Tasks;

using ArmoniK.Api.Common.Channel.Utils;
using ArmoniK.Api.Common.Utils;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;
using ArmoniK.Api.gRPC.V1.Worker;

using Grpc.Core;

using JetBrains.Annotations;

using Microsoft.Extensions.Logging;

namespace ArmoniK.Api.Worker.Worker;

[PublicAPI]
public class WorkerStreamWrapper : gRPC.V1.Worker.Worker.WorkerBase, IAsyncDisposable
{
  private readonly ChannelBase                  channel_;
  private readonly Agent.AgentClient            client_;
  private readonly ILoggerFactory               loggerFactory_;
  public           ILogger<WorkerStreamWrapper> logger_;

  public WorkerStreamWrapper(ILoggerFactory      loggerFactory,
                             GrpcChannelProvider provider)
  {
    logger_        = loggerFactory.CreateLogger<WorkerStreamWrapper>();
    loggerFactory_ = loggerFactory;

    channel_ = provider.Get();

    client_ = new Agent.AgentClient(channel_);
  }

  public async ValueTask DisposeAsync()
    => await channel_.ShutdownAsync()
                     .ConfigureAwait(false);

  public sealed override async Task<ProcessReply> Process(IAsyncStreamReader<ProcessRequest> requestStream,
                                                          ServerCallContext                  context)
  {
    Output output;
    {
      await using var taskHandler = await TaskHandler.Create(requestStream,
                                                             client_,
                                                             loggerFactory_,
                                                             context.CancellationToken)
                                                     .ConfigureAwait(false);

      using var _ = logger_.BeginNamedScope("Execute task",
                                            ("taskId", taskHandler.TaskId),
                                            ("sessionId", taskHandler.SessionId));
      logger_.LogDebug("Execute Process");
      output = await Process(taskHandler)
                 .ConfigureAwait(false);
    }
    return new ProcessReply
           {
             Output = output,
           };
  }

  public virtual Task<Output> Process(ITaskHandler taskHandler)
    => throw new RpcException(new Status(StatusCode.Unimplemented,
                                         ""));

  public override Task<HealthCheckReply> HealthCheck(Empty             request,
                                                     ServerCallContext context)
    => Task.FromResult(new HealthCheckReply
                       {
                         Status = HealthCheckReply.Types.ServingStatus.Serving,
                       });
}
