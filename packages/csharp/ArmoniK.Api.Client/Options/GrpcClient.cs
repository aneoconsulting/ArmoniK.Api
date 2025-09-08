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

using ArmoniK.Api.Client.Submitter;
using ArmoniK.Utils.DocAttribute;

using JetBrains.Annotations;

namespace ArmoniK.Api.Client.Options
{
  /// <summary>
  ///   Options for creating a gRPC Client with <see cref="GrpcChannelFactory" />
  /// </summary>
  [ExtractDocumentation("Options for GrpcClient")]
  [PublicAPI]
  public class GrpcClient
  {
    /// <summary>
    ///   Path to the section containing the values in the configuration object
    /// </summary>
    public const string SettingSection = nameof(GrpcClient);

    /// <summary>
    ///   Endpoint for sending requests
    /// </summary>
    public string? Endpoint { get; set; }

    /// <summary>
    ///   Allow unsafe connections to the endpoint (without SSL), defaults to false
    /// </summary>
    public bool AllowUnsafeConnection { get; set; }

    /// <summary>
    ///   Path to the certificate file in pem format
    /// </summary>
    public string CertPem { get; set; } = "";

    /// <summary>
    ///   Path to the key file in pem format
    /// </summary>
    public string KeyPem { get; set; } = "";

    /// <summary>
    ///   Path to the certificate file in p12/pfx format
    /// </summary>
    public string CertP12 { get; set; } = "";

    /// <summary>
    ///   Path to the Certificate Authority file in pem format
    /// </summary>
    public string CaCert { get; set; } = "";

    /// <summary>
    ///   Override the endpoint name during SSL verification. This option is only used when AllowUnsafeConnection is true and
    ///   only when the runtime is .NET Framework.
    ///   Automatic target name by default. Should be overriden by the right name to reduce performance cost.
    /// </summary>
    public string OverrideTargetName { get; set; } = "";


    /// <summary>
    ///   True if the options specify a client certificate
    /// </summary>
    public bool HasClientCertificate
      => !string.IsNullOrWhiteSpace(CertP12) || !(string.IsNullOrWhiteSpace(CertPem) || string.IsNullOrWhiteSpace(KeyPem));

    /// <summary>
    ///   KeepAliveTime is the time after which the connection will be kept alive.
    /// </summary>
    public TimeSpan KeepAliveTime { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>
    ///   KeepAliveTimeInterval is the interval at which the connection will be kept alive.
    /// </summary>
    public TimeSpan KeepAliveTimeInterval { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>
    ///   MaxIdleTime is the maximum idle time after which the connection will be closed.
    /// </summary>
    public TimeSpan MaxIdleTime { get; set; } = TimeSpan.FromMinutes(5);

    /// <summary>
    ///   MaxAttempts is a property that gets and sets the maximum number of attempts to retry an operation.
    /// </summary>
    public int MaxAttempts { get; set; } = 5;

    /// <summary>
    ///   The backoff will be multiplied by this multiplier after each retry attempt and will increase exponentially when the
    ///   multiplier is greater than 1.
    /// </summary>
    public double BackoffMultiplier { get; set; } = 1.5;

    /// <summary>
    ///   InitialBackOff is a property that gets and sets the initial backOff time for retrying an operation.
    /// </summary>
    public TimeSpan InitialBackOff { get; set; } = TimeSpan.FromSeconds(1);

    /// <summary>
    ///   MaxBackOff is a property that gets and sets the maximum backOff time for retrying an operation.
    /// </summary>
    public TimeSpan MaxBackOff { get; set; } = TimeSpan.FromSeconds(5);

    /// <summary>
    ///   Timeout for grpc requests. Defaults to no timeout.
    /// </summary>
    public TimeSpan RequestTimeout { get; set; } = Timeout.InfiniteTimeSpan;

    /// <summary>
    ///   Which HttpMessageHandler to use.
    ///   Valid options:
    ///   - `HttpClientHandler`
    ///   - `WinHttpHandler`
    ///   - `GrpcWebHandler`
    ///   If the handler is not set, the best one will be used.
    /// </summary>
    public string HttpMessageHandler { get; set; } = "";

    /// <summary>
    ///   Proxy configuration.
    ///   If empty, the default proxy configuration is used.
    ///   If "none", proxy is disabled.
    ///   If "system", the system proxy is used
    ///   Otherwise, it is the URL of the proxy to use
    /// </summary>
    public string Proxy { get; set; } = "";

    /// <summary>
    ///   Username used for proxy authentication
    /// </summary>
    public string ProxyUsername { get; set; } = "";

    /// <summary>
    ///   Password used for proxy authentication
    /// </summary>
    public string ProxyPassword { get; set; } = "";

    /// <summary>
    ///   Enable the option SO_REUSE_UNICASTPORT upon socket opening to limit port exhaustion
    /// </summary>
    public bool ReusePorts { get; set; } = true;
  }
}
