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

using ArmoniK.Api.Client.Submitter;

using JetBrains.Annotations;

namespace ArmoniK.Api.Client.Options
{
  /// <summary>
  ///   Options for creating a gRPC Client with <see cref="GrpcChannelFactory" />
  /// </summary>
  [PublicAPI]
  public class GrpcClient
  {
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
    ///   KeepAliveTime is the time in milliseconds after which the connection will be kept alive.
    /// </summary>
    public int KeepAliveTime { get; set; } = 30000;

    /// <summary>
    ///   KeepAliveTimeInterval is the interval in milliseconds at which the connection will be kept alive.
    /// </summary>
    public int KeepAliveTimeInterval { get; set; } = 30000;

    /// <summary>
    ///   MaxIdleTime is the maximum idle time in minutes after which the connection will be closed.
    /// </summary>
    public int MaxIdleTime { get; set; } = 5;


    /// <summary>
    ///   MaxAttempts is a property that gets and sets the maximum number of attempts to retry an operation.
    /// </summary>
    public int MaxAttempts { get; set; } = 5;

    /// <summary>
    ///   InitialBackOff is a property that gets and sets the initial backOff time in seconds for retrying an operation.
    /// </summary>
    public int InitialBackOff { get; set; } = 1;

    /// <summary>
    ///   MaxBackOff is a property that gets and sets the maximum backOff time in seconds for retrying an operation.
    /// </summary>
    public int MaxBackOff { get; set; } = 5;
  }
}
