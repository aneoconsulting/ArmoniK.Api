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
  }
}
