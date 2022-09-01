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

using System.Net.Sockets;

using ArmoniK.Api.Common.Options;
using ArmoniK.Api.Common.Utils;

using Grpc.Core;
using Grpc.Net.Client;

using JetBrains.Annotations;

using Microsoft.Extensions.Logging;

using GrpcChannel = ArmoniK.Api.Common.Options.GrpcChannel;

namespace ArmoniK.Api.Common.Channel.Utils;

[UsedImplicitly]
public sealed class GrpcChannelProvider : IAsyncDisposable
{
  private readonly string                       address_;
  private readonly ILogger<GrpcChannelProvider> logger_;
  private readonly GrpcChannel                  options_;
  private          NetworkStream?               networkStream_;
  private          Socket?                      socket_;

  public GrpcChannelProvider(GrpcChannel                  options,
                             ILogger<GrpcChannelProvider> logger)
  {
    options_ = options;
    logger_  = logger;
    address_ = options_.Address ?? throw new InvalidOperationException();
    logger.LogDebug("Channel created for address : {address}",
                    address_);
  }

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
