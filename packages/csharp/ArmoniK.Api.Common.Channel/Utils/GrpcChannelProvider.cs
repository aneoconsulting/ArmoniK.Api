// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2024. All rights reserved.
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

using System.Net.Sockets;

using ArmoniK.Api.Common.Options;
using ArmoniK.Api.Common.Utils;

using Grpc.Core;
using Grpc.Net.Client;

using JetBrains.Annotations;

using Microsoft.AspNetCore.Server.Kestrel.Core;
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

  private static ChannelBase BuildWebGrpcChannel(string  address,
                                                 ILogger logger)
  {
    using var _ = logger.LogFunction();
    return Grpc.Net.Client.GrpcChannel.ForAddress(address);
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
                                                    HttpHandler = socketsHttpHandler,
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

  /// <summary>
  ///   Sets listen socket in kestrel according to gRPC Channel
  /// </summary>
  /// <returns>A KestrelServerOptions object set with the appropriate socket </returns>
  /// <exception cref="InvalidOperationException">when socket type is unknown</exception>
  public void KestrelOptionsProvider(KestrelServerOptions serverOptions)
  {
    switch (options_?.SocketType)
    {
      case GrpcSocketType.UnixDomainSocket:
        if (File.Exists(address_))
        {
          File.Delete(address_);
        }
        serverOptions.ListenUnixSocket(address_,
                                       listenOptions => listenOptions.Protocols = HttpProtocols.Http2);
        break;
      case GrpcSocketType.Tcp:
        var success = int.TryParse(address_,
                                   out var port);
        if (success)
        {
          serverOptions.ListenAnyIP(port, listenOptions =>
                                            listenOptions.Protocols = HttpProtocols.Http2);
        }
        else
        {
          throw new Exception($"Could not parse {nameof(address_)} to a valid port number");
        }

        break;
      default:
        throw new InvalidOperationException("Socket type unknown");
    }
  }
}
