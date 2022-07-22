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
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Security.Cryptography.X509Certificates;

using ArmoniK.Api.Client.Options;

using Grpc.Core;
using Grpc.Net.Client;

using JetBrains.Annotations;

namespace ArmoniK.Api.Client.Submitter
{

  /// <summary>
  /// Factory for creating a secure GrpcChannel
  /// </summary>
  [PublicAPI]
  public static class GrpcChannelFactory
  {

    /// <summary>
    /// Creates the GrpcChannel
    /// </summary>
    /// <param name="optionsGrpcClient">Options for the creation of the channel</param>
    /// <returns>
    /// The initialized GrpcChannel
    /// </returns>
    /// <exception cref="InvalidOperationException">Endpoint passed through options is missing</exception>
    public static GrpcChannel CreateChannel(GrpcClient optionsGrpcClient)
    {
      if (string.IsNullOrEmpty(optionsGrpcClient.Endpoint))
      {
        throw new InvalidOperationException($"{nameof(optionsGrpcClient.Endpoint)} should not be null or empty");
      }

      var uri = new Uri(optionsGrpcClient.Endpoint);

      var credentials = uri.Scheme == Uri.UriSchemeHttps
                          ? new SslCredentials()
                          : ChannelCredentials.Insecure;
      HttpClientHandler httpClientHandler = new HttpClientHandler();

      if (optionsGrpcClient.AllowUnsafeConnection)
      {
        httpClientHandler.ServerCertificateCustomValidationCallback = HttpClientHandler.DangerousAcceptAnyServerCertificateValidator;
        AppContext.SetSwitch("System.Net.Http.SocketsHttpHandler.Http2UnencryptedSupport",
                             true);
      }

      if (!string.IsNullOrEmpty(optionsGrpcClient.CertPem) && string.IsNullOrEmpty(optionsGrpcClient.KeyPem))
      {
        var cert = X509Certificate2.CreateFromPemFile(optionsGrpcClient.CertPem,
                                                      optionsGrpcClient.KeyPem);

        // Resolve issue with Windows on pem bug with windows
        // https://github.com/dotnet/runtime/issues/23749#issuecomment-388231655
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
          var originalCert = cert;
          cert = new X509Certificate2(cert.Export(X509ContentType.Pkcs12));
          originalCert.Dispose();
        }

        httpClientHandler.ClientCertificates.Add(cert);
      }

      var channelOptions = new GrpcChannelOptions()
                           {
                             Credentials = credentials,
                             HttpHandler = httpClientHandler,
                           };

      var channel = GrpcChannel.ForAddress(optionsGrpcClient.Endpoint,
                                           channelOptions);

      return channel;
    }
  }
}
