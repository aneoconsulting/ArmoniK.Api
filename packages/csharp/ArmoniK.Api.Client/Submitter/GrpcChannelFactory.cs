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
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Net.Security;
using System.Runtime.InteropServices;
using System.Security.Authentication;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.Client.Options;

using Grpc.Core;
using Grpc.Net.Client;
using Grpc.Net.Client.Configuration;
using Grpc.Net.Client.Web;

using JetBrains.Annotations;

using Microsoft.Extensions.Logging;

using Org.BouncyCastle.Crypto;
using Org.BouncyCastle.OpenSsl;
using Org.BouncyCastle.Pkcs;
using Org.BouncyCastle.Security;
using Org.BouncyCastle.X509;

using X509Certificate = Org.BouncyCastle.X509.X509Certificate;

namespace ArmoniK.Api.Client.Submitter
{
  /// <summary>
  ///   Factory for creating a secure GrpcChannel
  /// </summary>
  [PublicAPI]
  public static class GrpcChannelFactory
  {
    /// <summary>
    ///   Get the server certificate validation callback
    /// </summary>
    /// <param name="insecure">Whether validation should be performed</param>
    /// <param name="caCert">Root certificate to validate Server cert against</param>
    /// <returns>Validation callback</returns>
    private static Func<HttpRequestMessage, X509Certificate2, X509Chain, SslPolicyErrors, bool>? GetServerCertificateValidationCallback(bool             insecure,
                                                                                                                                        X509Certificate? caCert)
    {
      // If insecure, allow any certificates
      if (insecure)
      {
        AppContext.SetSwitch("System.Net.Http.SocketsHttpHandler.Http2UnencryptedSupport",
                             true);
        return (request,
                certificate2,
                certChain,
                sslPolicyErrors) => true;
      }

      // If no CaCert, just use the standard validation against the machine certificate store
      if (caCert is null)
      {
        return null;
      }

      // Validate against a specific certificate
      var authority = new X509Certificate2(DotNetUtilities.ToX509Certificate(caCert));

      // Implementation inspired from https://stackoverflow.com/a/52926718
      return (request,
              certificate2,
              certChain,
              sslPolicyErrors) =>
             {
               // If there is any error other than untrusted root or partial chain, fail the validation
               if ((sslPolicyErrors & ~SslPolicyErrors.RemoteCertificateChainErrors) != 0)
               {
                 return false;
               }

               // If there is any error other than untrusted root or partial chain, fail the validation
               if (certChain.ChainStatus.Any(status => status.Status is not X509ChainStatusFlags.UntrustedRoot and not X509ChainStatusFlags.PartialChain))
               {
                 return false;
               }

               // Disable some extensive checks that would fail on the authority that is not in store
               certChain.ChainPolicy.RevocationMode    = X509RevocationMode.NoCheck;
               certChain.ChainPolicy.VerificationFlags = X509VerificationFlags.AllowUnknownCertificateAuthority;

               // Add unknown authority to the store
               certChain.ChainPolicy.ExtraStore.Add(authority);

               // Check if the chain is valid for the actual server certificate (ie: trusted)
               if (!certChain.Build(certificate2))
               {
                 return false;
               }

               // Check that the chain root is actually the specified authority (caCert)
               return certChain.ChainElements.Cast<X509ChainElement>()
                               .Any(x => x.Certificate.Thumbprint == authority.Thumbprint);
             };
    }

    /// <summary>
    ///   Creates a HttpMessageHandler for the current platform
    /// </summary>
    /// <param name="https">Whether https is used or not</param>
    /// <param name="insecure">Whether the Server Certificate should be validated or not</param>
    /// <param name="caCert">Root certificate to validate the server certificate against</param>
    /// <param name="clientCert">Client certificate to be used for mTLS</param>
    /// <param name="handlerType">Which HttpMessageHandler type to use</param>
    /// <param name="logger">Optional logger</param>
    /// <returns>HttpMessageHandler</returns>
    private static HttpMessageHandler CreateHttpMessageHandler(bool              https,
                                                               bool              insecure,
                                                               X509Certificate?  caCert,
                                                               X509Certificate2? clientCert,
                                                               HandlerType       handlerType,
                                                               ILogger?          logger = null)
    {
      Func<HttpRequestMessage, X509Certificate2, X509Chain, SslPolicyErrors, bool>? validationCallback = null;
      SslProtocols                                                                  sslProtocols       = default;

      if (https)
      {
        sslProtocols = GetSslProtocols();
        validationCallback = GetServerCertificateValidationCallback(insecure,
                                                                    caCert);
      }

      if (handlerType is HandlerType.Http)
      {
        var httpHandler = new HttpClientHandler();
        if (!https)
        {
          return httpHandler;
        }

        httpHandler.SslProtocols                              = sslProtocols;
        httpHandler.ServerCertificateCustomValidationCallback = validationCallback;

        if (clientCert is not null)
        {
          httpHandler.ClientCertificates.Add(clientCert);
        }

        return httpHandler;
      }

      if (!https && handlerType is HandlerType.Win)
      {
        throw new InvalidOperationException("WinHttpHandler does not support plain text HTTP/2");
      }

      var winHandler = new WinHttpHandler();

      if (https)
      {
        winHandler.SslProtocols                        = sslProtocols;
        winHandler.ServerCertificateValidationCallback = validationCallback;

        if (clientCert is not null)
        {
          winHandler.ClientCertificates.Add(clientCert);
        }
      }

      if (handlerType is HandlerType.Web)
      {
        return new GrpcWebHandler(winHandler);
      }

      return winHandler;
    }

    /// <summary>
    ///   Get the list of supported SSL protocols, and enable TLS1.3 if possible
    /// </summary>
    /// <returns>SSL protocols enum</returns>
    private static SslProtocols GetSslProtocols()
    {
      try
      {
        // try TLS1.3
        ServicePointManager.SecurityProtocol |= (SecurityProtocolType)12288 | SecurityProtocolType.Tls12 | SecurityProtocolType.Tls11 | SecurityProtocolType.Tls;
        return (SslProtocols)12288 | SslProtocols.Tls12 | SslProtocols.Tls11 | SslProtocols.Tls;
      }
      catch (NotSupportedException)
      {
        ServicePointManager.SecurityProtocol |= SecurityProtocolType.Tls12 | SecurityProtocolType.Tls11 | SecurityProtocolType.Tls;
        return SslProtocols.Tls12 | SslProtocols.Tls11 | SslProtocols.Tls;
      }
    }

    /// <summary>
    ///   Creates the GrpcChannel
    /// </summary>
    /// <param name="optionsGrpcClient">Options for the creation of the channel</param>
    /// <param name="handlerType">Which HttpMessageHandler type to use</param>
    /// <param name="logger">Optional logger</param>
    /// <returns>
    ///   The initialized GrpcChannel
    /// </returns>
    /// <exception cref="InvalidOperationException">Endpoint passed through options is missing</exception>
    private static GrpcChannel CreateChannelInternal(GrpcClient  optionsGrpcClient,
                                                     HandlerType handlerType,
                                                     ILogger?    logger)
    {
      if (string.IsNullOrEmpty(optionsGrpcClient.Endpoint))
      {
        throw new InvalidOperationException($"{nameof(optionsGrpcClient.Endpoint)} should not be null or empty");
      }

      var serviceConfig = new ServiceConfig
                          {
                            MethodConfigs =
                            {
                              new MethodConfig
                              {
                                Names =
                                {
                                  MethodName.Default,
                                },
                                RetryPolicy = new RetryPolicy
                                              {
                                                MaxAttempts       = optionsGrpcClient.MaxAttempts,
                                                InitialBackoff    = optionsGrpcClient.InitialBackOff,
                                                MaxBackoff        = optionsGrpcClient.MaxBackOff,
                                                BackoffMultiplier = optionsGrpcClient.BackoffMultiplier,
                                                RetryableStatusCodes =
                                                {
                                                  StatusCode.Unavailable,
                                                  StatusCode.Aborted,
                                                  StatusCode.Unknown,
                                                },
                                              },
                              },
                            },
                          };

      X509Certificate? caCert = null;

      // Parse CaCert from file
      if (!string.IsNullOrWhiteSpace(optionsGrpcClient.CaCert) && !optionsGrpcClient.AllowUnsafeConnection)
      {
        if (!File.Exists(optionsGrpcClient.CaCert))
        {
          throw new FileNotFoundException("Couldn't find specified CA certificate",
                                          optionsGrpcClient.CaCert);
        }

        var parser = new X509CertificateParser();
        using var stream = File.Open(optionsGrpcClient.CaCert,
                                     FileMode.Open,
                                     FileAccess.Read,
                                     FileShare.Read);
        caCert = parser.ReadCertificate(stream);
      }

      var uri = new Uri(optionsGrpcClient.Endpoint!);

      var credentials = uri.Scheme == Uri.UriSchemeHttps
                          ? ChannelCredentials.SecureSsl
                          : ChannelCredentials.Insecure;
      var clientCert = optionsGrpcClient.HasClientCertificate
                         ? GetCertificate(optionsGrpcClient)
                         : null;

      switch (handlerType)
      {
        case HandlerType.Http:
          logger?.LogDebug("Create HttpClientHandler() for {Endpoint}",
                           optionsGrpcClient.Endpoint);
          break;
        case HandlerType.Win:
          logger?.LogDebug("Create WinHttpHandler() for {Endpoint}",
                           optionsGrpcClient.Endpoint);
          break;
        case HandlerType.Web:
          logger?.LogDebug("Create GrpcWebHandler(WinHttpHandler()) for {Endpoint}",
                           optionsGrpcClient.Endpoint);
          break;
      }

      var httpHandler = CreateHttpMessageHandler(uri.Scheme == Uri.UriSchemeHttps,
                                                 optionsGrpcClient.AllowUnsafeConnection,
                                                 caCert,
                                                 clientCert,
                                                 handlerType,
                                                 logger);

      // Warn that RequestTimeout is not supported.
      // If required, it could be easily implemented with a DelegatingHandler and a cancellationToken delayed cancellation
      if (optionsGrpcClient.RequestTimeout != Timeout.InfiniteTimeSpan)
      {
        logger?.LogWarning("Request Timeout is not supported, no timeout is applied");
      }

      var sp = ServicePointManager.FindServicePoint(new Uri(optionsGrpcClient.Endpoint!));

      sp.SetTcpKeepAlive(true,
                         (int)optionsGrpcClient.KeepAliveTime.TotalMilliseconds,
                         (int)optionsGrpcClient.KeepAliveTimeInterval.TotalMilliseconds);

      sp.MaxIdleTime = (int)optionsGrpcClient.MaxIdleTime.TotalMilliseconds;

      var channelOptions = new GrpcChannelOptions
                           {
                             Credentials       = credentials,
                             DisposeHttpClient = true,
                             ServiceConfig     = serviceConfig,
                           };

      if (handlerType is HandlerType.Web)
      {
        // If GrpcWebHandler is used, we must provide it an HttpClient to overcome a check issue
        channelOptions.HttpClient = new HttpClient(httpHandler);
      }
      else
      {
        // If using a WinHttpHandler, we must set the httpHandler directly, otherwise, HTTP/2 is not properly supported
        channelOptions.HttpHandler = httpHandler;
      }

      return GrpcChannel.ForAddress(optionsGrpcClient.Endpoint!,
                                    channelOptions);
    }

    /// <summary>
    ///   Creates the GrpcChannel
    /// </summary>
    /// <param name="optionsGrpcClient">Options for the creation of the channel</param>
    /// <param name="logger">Optional logger</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>
    ///   The initialized GrpcChannel
    /// </returns>
    /// <exception cref="InvalidOperationException">Endpoint passed through options is missing</exception>
    public static Task<GrpcChannel> CreateChannelAsync(GrpcClient        optionsGrpcClient,
                                                       ILogger?          logger            = null,
                                                       CancellationToken cancellationToken = default)
      => Task.FromResult(CreateChannel(optionsGrpcClient,
                                       logger));

    /// <summary>
    ///   Creates the GrpcChannel
    /// </summary>
    /// <param name="optionsGrpcClient">Options for the creation of the channel</param>
    /// <param name="logger">Optional logger</param>
    /// <returns>
    ///   The initialized GrpcChannel
    /// </returns>
    /// <exception cref="InvalidOperationException">Endpoint passed through options is missing</exception>
    public static GrpcChannel CreateChannel(GrpcClient optionsGrpcClient,
                                            ILogger?   logger = null)
    {
      if (!string.IsNullOrEmpty(optionsGrpcClient.OverrideTargetName))
      {
        logger?.LogWarning("OverrideTargetName is not supported");
      }

      // ReSharper disable once ConvertTypeCheckPatternToNullCheck
      if (ParseHandler(optionsGrpcClient.HttpMessageHandler) is HandlerType handlerType)
      {
        return CreateChannelInternal(optionsGrpcClient,
                                     handlerType,
                                     logger);
      }

      // If dotnet core (>= Net 5), we can use HttpClientHandler
      if (!RuntimeInformation.FrameworkDescription.StartsWith(".NET Framework"))
      {
        return CreateChannelInternal(optionsGrpcClient,
                                     HandlerType.Http,
                                     logger);
      }

      // If dotnet framework, we can use a plain WinHttpHandler.
      // WinHttpHandler supports gRPC on Windows 11 and Windows server 2022 only, and using TLS only.
      try
      {
        return CreateChannelInternal(optionsGrpcClient,
                                     HandlerType.Win,
                                     logger);
      }
      catch (InvalidOperationException e)
      {
        // If it is not supported (either plain text or earlier windows version),
        // we need to fallback to GrpcWebHandler that works on HTTP/1.1, but can be buggy with large or bidirectional streams
        logger?.LogWarning(e,
                           "Falling back to gRPC Web that does not fully support gRPC streams");
        return CreateChannelInternal(optionsGrpcClient,
                                     HandlerType.Web,
                                     logger);
      }
    }

    /// <summary>
    ///   Get the certificate in PFX format from the given options.
    ///   Loads the certificate file directly if <paramref name="optionsGrpcClient.CertP12" /> is specified, otherwise creates
    ///   it from the pem formatted files <paramref name="optionsGrpcClient.CertPem" /> and
    ///   <paramref name="optionsGrpcClient.KeyPem" />
    /// </summary>
    /// <param name="optionsGrpcClient">Client option</param>
    /// <returns>The PFX formatted client certificate</returns>
    /// <exception cref="FileNotFoundException">
    ///   The P12 certificate is specified but not found, or either the Pem cert or key
    ///   are not found
    /// </exception>
    /// <exception cref="InvalidOperationException">No certificate was specified in options</exception>
    /// <exception cref="CryptographicException">The key could not be retrieved from the key file</exception>
    public static X509Certificate2 GetCertificate(GrpcClient optionsGrpcClient)
    {
      if (!string.IsNullOrEmpty(optionsGrpcClient.CertP12))
      {
        if (!File.Exists(optionsGrpcClient.CertP12))
        {
          throw new FileNotFoundException("Couldn't find specified P12 client certificate",
                                          optionsGrpcClient.CertP12);
        }

        return new X509Certificate2(optionsGrpcClient.CertP12);
      }

      if (string.IsNullOrEmpty(optionsGrpcClient.CertPem) || string.IsNullOrEmpty(optionsGrpcClient.KeyPem))
      {
        throw new InvalidOperationException("Cannot find requested certificate from options");
      }

      X509Certificate cert;
      using (var reader = new FileStream(optionsGrpcClient.CertPem,
                                         FileMode.Open,
                                         FileAccess.Read,
                                         FileShare.Read))
      {
        cert = new X509CertificateParser().ReadCertificate(reader);
      }

      var store = new Pkcs12StoreBuilder().Build();
      using (var reader = new FileStream(optionsGrpcClient.KeyPem,
                                         FileMode.Open,
                                         FileAccess.Read,
                                         FileShare.Read))
      using (var textReader = new StreamReader(reader))
      {
        var                     pemReader = new PemReader(textReader);
        AsymmetricKeyParameter? key;

        do
        {
          key = pemReader.ReadObject() switch
                {
                  null                            => throw new CryptographicException("Key could not be retrieved from file"),
                  AsymmetricCipherKeyPair keyPair => keyPair.Private,
                  AsymmetricKeyParameter keyParam => keyParam,
                  _                               => null,
                };
        } while (key is null);

        store.SetKeyEntry("alias",
                          new AsymmetricKeyEntry(key),
                          new X509CertificateEntry[]
                          {
                            new(cert),
                          });
      }

      using var pkcs = new MemoryStream();
      store.Save(pkcs,
                 Array.Empty<char>(),
                 new SecureRandom());
      return new X509Certificate2(pkcs.ToArray(),
                                  string.Empty,
                                  X509KeyStorageFlags.Exportable);
    }

    private static HandlerType? ParseHandler(string handler)
      => handler.ToLower() switch
         {
           ""                                                        => null,
           "httpclienthandler" or "httpclient" or "http" or "client" => HandlerType.Http,
           "winhttphandler" or "winhttp" or "win"                    => HandlerType.Win,
           "grpcwebhandler" or "grpcweb" or "web"                    => HandlerType.Web,
           _                                                         => throw new ArgumentException($"Invalid HandlerType: {handler}"),
         };

    private enum HandlerType
    {
      /// <summary>
      ///   HttpClientHandler()
      /// </summary>
      Http,

      /// <summary>
      ///   WinHttpHandler()
      /// </summary>
      Win,

      /// <summary>
      ///   GrpcWebHandler(WinHttpHandler())
      /// </summary>
      Web,
    }
  }
}
