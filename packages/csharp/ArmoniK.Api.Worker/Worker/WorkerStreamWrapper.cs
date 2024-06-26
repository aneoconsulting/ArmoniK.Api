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
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.Common.Channel.Utils;
using ArmoniK.Api.Common.Options;
using ArmoniK.Api.Common.Utils;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;
using ArmoniK.Api.gRPC.V1.Worker;
using ArmoniK.Utils;

using Grpc.Core;

using JetBrains.Annotations;

using Microsoft.Extensions.Logging;

namespace ArmoniK.Api.Worker.Worker;

/// <summary>
///   Wrapper implementation that provide a simpler interface to use for tasks implementations in C#
/// </summary>
[PublicAPI]
public class WorkerStreamWrapper : gRPC.V1.Worker.Worker.WorkerBase, IAsyncDisposable
{
  private const    int               AbortReturnCode = 1;
  private readonly ChannelBase       channel_;
  private readonly Agent.AgentClient client_;
  private readonly ComputePlane      computePlaneOptions_;
  private readonly ILoggerFactory    loggerFactory_;

  /// <summary>
  ///   Logger used for printing logs during task execution
  /// </summary>
  [PublicAPI]
  public ILogger<WorkerStreamWrapper> logger_;

  /// <summary>
  ///   Instantiate a simpler interface to use for tasks implementations
  /// </summary>
  /// <param name="loggerFactory">LoggerFactory to create loggers</param>
  /// <param name="computePlaneOptions">Options for the ComputePlane</param>
  /// <param name="provider">gRPC channel provider to create channels with the Agent</param>
  public WorkerStreamWrapper(ILoggerFactory      loggerFactory,
                             ComputePlane        computePlaneOptions,
                             GrpcChannelProvider provider)
  {
    logger_              = loggerFactory.CreateLogger<WorkerStreamWrapper>();
    loggerFactory_       = loggerFactory;
    computePlaneOptions_ = computePlaneOptions;

    channel_ = provider.Get();

    client_ = new Agent.AgentClient(channel_);
  }

  /// <summary>
  ///   Instantiate a simpler interface to use for tasks implementations
  /// </summary>
  /// <param name="loggerFactory">LoggerFactory to create loggers</param>
  /// <param name="provider">gRPC channel provider to create channels with the Agent</param>
  [Obsolete("Superseded by WorkerStreamWrapper(ILoggerFactory, Options::ComputePlane, GrpcChannelProvider)")]
  public WorkerStreamWrapper(ILoggerFactory      loggerFactory,
                             GrpcChannelProvider provider)
    : this(loggerFactory,
           new ComputePlane(),
           provider)
  {
  }

  /// <inheritdoc />
  public async ValueTask DisposeAsync()
    => await channel_.ShutdownAsync()
                     .ConfigureAwait(false);


  /// <inheritdoc />
  public sealed override async Task<ProcessReply> Process(ProcessRequest    request,
                                                          ServerCallContext context)
  {
    var output = new Output();

    await using var taskHandler = new TaskHandler(request,
                                                  client_,
                                                  loggerFactory_,
                                                  context.CancellationToken);

    using var _ = logger_.BeginNamedScope("Execute task",
                                          ("taskId", taskHandler.TaskId),
                                          ("sessionId", taskHandler.SessionId));
    logger_.LogDebug("Execute Process");
    var process = ProcessAsync(taskHandler,
                               context.CancellationToken);

    // Wait for process or cancellationToken
    var task = await Task.WhenAny(process,
                                  context.CancellationToken.AsTask())
                         .ConfigureAwait(false);

    // Check normal completion
    if (ReferenceEquals(task,
                        process))
    {
      output = await process.ConfigureAwait(false);
    }
    else
    {
      logger_.LogInformation("RPC request was aborted");

      if (computePlaneOptions_.AbortAfter > TimeSpan.Zero)
      {
        // Request has been cancelled
        // Wait for process to be properly cancelled, or small delay
        task = await Task.WhenAny(process,
                                  Task.Delay(computePlaneOptions_.AbortAfter))
                         .ConfigureAwait(false);
      }

      // Check cancelled completion of process
      if (ReferenceEquals(task,
                          process))
      {
        output = await process.ConfigureAwait(false);
      }
      else
      {
        logger_.LogError("Stopping the worker as processing has not stopped");

        // Task did not finish upon cancellation
        // Abort the whole worker to ensure the task does not continue running
        Environment.Exit(AbortReturnCode);
      }
    }

    return new ProcessReply
           {
             Output = output,
           };
  }

  /// <summary>
  ///   User defined computations
  /// </summary>
  /// <param name="taskHandler">Handler to access input data and task capabilities</param>
  /// <returns>
  ///   The output of the computational task
  /// </returns>
  /// <exception cref="RpcException">when method is not overwritten</exception>
  public virtual Task<Output> Process(ITaskHandler taskHandler)
    => throw new RpcException(new Status(StatusCode.Unimplemented,
                                         ""));

  /// <summary>
  ///   User defined computations
  ///   Calls <see cref="Process(ArmoniK.Api.gRPC.V1.Worker.ProcessRequest,Grpc.Core.ServerCallContext)" /> if not overriden.
  /// </summary>
  /// <param name="taskHandler">Handler to access input data and task capabilities</param>
  /// <param name="cancellationToken">Token used to cancel the execution of the method</param>
  /// <returns>
  ///   The output of the computational task
  /// </returns>
  public virtual Task<Output> ProcessAsync(ITaskHandler      taskHandler,
                                           CancellationToken cancellationToken)
  {
    _ = cancellationToken;
    return Process(taskHandler);
  }

  /// <inheritdoc />
  public override Task<HealthCheckReply> HealthCheck(Empty             request,
                                                     ServerCallContext context)
    => Task.FromResult(new HealthCheckReply
                       {
                         Status = HealthCheckReply.Types.ServingStatus.Serving,
                       });
}
