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

using System;

using ArmoniK.Utils.DocAttribute;

using JetBrains.Annotations;

namespace ArmoniK.Api.Common.Options;

/// <summary>
///   Options to configure a channel from gRPC
/// </summary>
[ExtractDocumentation("Options for GrpcChannel")]
[PublicAPI]
public class GrpcChannel
{
  /// <summary>
  ///   Address or path of the resource used to communicate for this gRPC Channel
  /// </summary>
  public string Address { get; set; } = "/tmp/armonik.sock";

  /// <summary>
  ///   Type of gRPC Socket used
  /// </summary>
  public GrpcSocketType SocketType { get; set; } = GrpcSocketType.UnixDomainSocket;

  /// <summary>
  ///   Keep-alive ping timeout for http2 connections
  /// </summary>
  public TimeSpan KeepAlivePingTimeOut { get; set; }

  /// <summary>
  ///   Keep-alive timeout
  /// </summary>
  public TimeSpan KeepAliveTimeOut { get; set; }

  /// <summary>
  ///   Retry policy options for this gRPC channel.
  /// </summary>
  public GrpcChannelRetryPolicy RetryPolicy { get; set; } = new();

  /// <summary>
  ///   Returns a new <see cref="GrpcChannel" /> with the given <see cref="GrpcChannelRetryPolicy" /> applied.
  /// </summary>
  /// <param name="retryPolicy">The retry policy to apply.</param>
  /// <returns>A new <see cref="GrpcChannel" /> instance with the updated retry policy.</returns>
  public GrpcChannel WithRetryPolicy(GrpcChannelRetryPolicy retryPolicy)
  {
    RetryPolicy = retryPolicy;
    return this;
  }
}
