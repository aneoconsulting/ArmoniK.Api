// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2026. All rights reserved.
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

using System.Net.Sockets;

using ArmoniK.Api.Common.Options;
using ArmoniK.Api.Common.Utils;

using Grpc.Core;
using Grpc.Net.Client;
using Grpc.Net.Client.Configuration;

using JetBrains.Annotations;

using Microsoft.Extensions.Logging;

using GrpcChannel = ArmoniK.Api.Common.Options.GrpcChannel;

namespace ArmoniK.Api.Common.Channel.Utils;

/// <summary>
///   Provides a built gRPC Channel from given options
/// </summary>
[UsedImplicitly]
public sealed class GrpcChannelProvider : IAsyncDisposable
{
  private readonly string                       address_;
  private readonly ILogger<GrpcChannelProvider> logger_;
  private readonly GrpcChannel                  options_;
  private          NetworkStream?               networkStream_;
  private          Socket?                      socket_;

  /// <summary>
  ///   Instantiate a <see cref="GrpcChannelProvider" /> that creates a gRPC channel
  /// </summary>
  /// <param name="options">Options to configure the creation of the gRPC channel</param>
  /// <param name="logger">Logger that will produce logs</param>
  /// <exception cref="InvalidOperationException">when address is empty</exception>
  public GrpcChannelProvider(GrpcChannel                  options,
                             ILogger<GrpcChannelProvider> logger)
  {
    options_ = options;
    logger_  = logger;
    address_ = options_.Address ?? throw new InvalidOperationException();
    logger.LogDebug("Channel created for address : {address}",
                    address_);
  }

  /// <inheritdoc />
  public async ValueTask DisposeAsync()
  {
    socket_?.Close();
    socket_?.Dispose();
    if (networkStream_ != null)
    {
      await networkStream_.DisposeAsync()
                          .ConfigureAwait(false);
    }
  }

  /// <summary>
  ///   Creates the gRPC <see cref="ServiceConfig" /> with a retry policy for transient failures,
  ///   using values from the <see cref="GrpcChannel" /> options.
  ///   Retries on transient/recoverable status codes with exponential backoff.
  ///   See: https://learn.microsoft.com/en-us/aspnet/core/grpc/retries
  ///   See: https://grpc.github.io/grpc/core/md_doc_statuscodes.html
  /// </summary>
  private ServiceConfig BuildServiceConfig()
  {
    if (options_.RetryPolicy.MaxAttempts <= 1)
    {
      logger_.LogDebug("gRPC native retry policy is disabled (RetryMaxAttempts={maxAttempts})",
                       options_.RetryPolicy.MaxAttempts);
      return new ServiceConfig();
    }

    logger_.LogInformation("gRPC native retry policy enabled: MaxAttempts={maxAttempts}, InitialBackoff={initialBackoff}, MaxBackoff={maxBackoff}, Multiplier={multiplier}",
                           options_.RetryPolicy.MaxAttempts,
                           options_.RetryPolicy.InitialBackoff,
                           options_.RetryPolicy.MaxBackoff,
                           options_.RetryPolicy.BackoffMultiplier);

    return new ServiceConfig
           {
             MethodConfigs =
             {
               new MethodConfig
               {
                 Names       = { MethodName.Default },
                 RetryPolicy = new RetryPolicy
                               {
                                 MaxAttempts       = options_.RetryPolicy.MaxAttempts,
                                 InitialBackoff    = options_.RetryPolicy.InitialBackoff,
                                 MaxBackoff        = options_.RetryPolicy.MaxBackoff,
                                 BackoffMultiplier = options_.RetryPolicy.BackoffMultiplier,
                                 RetryableStatusCodes =
                                 {
                                   StatusCode.Unavailable,
                                   StatusCode.Internal,
                                   StatusCode.Aborted,
                                   StatusCode.ResourceExhausted,
                                 },
                               },
               },
             },
           };
  }

  private ChannelBase BuildWebGrpcChannel(string  address,
                                                ILogger logger)
  {
    using var _ = logger.LogFunction();
    return Grpc.Net.Client.GrpcChannel.ForAddress(address,
                                                  new GrpcChannelOptions
                                                  {
                                                    ServiceConfig = BuildServiceConfig(),
                                                  });
  }

  private ChannelBase BuildUnixSocketGrpcChannel(string  address,
                                                 ILogger logger)
  {
    using var _ = logger.LogFunction();

    var udsEndPoint = new UnixDomainSocketEndPoint(address);

    // Workaround for connectivity issue: https://github.com/grpc/grpc-dotnet/issues/2361#issuecomment-1895791020
    AppContext.SetSwitch("System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing",
                         true);

    var socketsHttpHandler = new SocketsHttpHandler
                             {
                               ConnectCallback = async (_,
                                                        cancellationToken) =>
                                                 {
                                                   socket_ = new Socket(AddressFamily.Unix,
                                                                        SocketType.Stream,
                                                                        ProtocolType.Unspecified);

                                                   try
                                                   {
                                                     await socket_.ConnectAsync(udsEndPoint,
                                                                                cancellationToken)
                                                                  .ConfigureAwait(false);
                                                     networkStream_ = new NetworkStream(socket_,
                                                                                        true);
                                                     return networkStream_;
                                                   }
                                                   catch
                                                   {
                                                     socket_.Dispose();
                                                     throw;
                                                   }
                                                 },
                             };

    return Grpc.Net.Client.GrpcChannel.ForAddress("http://localhost",
                                                  new GrpcChannelOptions
                                                  {
                                                    HttpHandler   = socketsHttpHandler,
                                                    ServiceConfig = BuildServiceConfig(),
                                                  });
  }

  /// <summary>
  ///   Access to the created gRPC Channel
  /// </summary>
  /// <returns>The created gRPC Channel</returns>
  /// <exception cref="InvalidOperationException">when socket type is unknown</exception>
  public ChannelBase Get()
  {
    switch (options_.SocketType)
    {
      case GrpcSocketType.Tcp:
        return BuildWebGrpcChannel(address_,
                                   logger_);
      case GrpcSocketType.UnixDomainSocket:
        return BuildUnixSocketGrpcChannel(address_,
                                          logger_);
      default:
        throw new InvalidOperationException();
    }
  }
}
