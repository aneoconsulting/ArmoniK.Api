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
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Security.Authentication;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Text;

using ArmoniK.Api.Client.Options;

using Grpc.Core;
using Grpc.Net.Client;

using JetBrains.Annotations;

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
    private static string GetRootCertificates()
    {
      var builder = new StringBuilder();
      var store   = new X509Store(StoreName.Root);
      store.Open(OpenFlags.ReadOnly);
      foreach (var mCert in store.Certificates)
      {
        builder.AppendLine($"# Issuer: {mCert.Issuer}\n# Subject: {mCert.Subject}\n# Label: {mCert.FriendlyName}\n# Serial: {mCert.SerialNumber}\n# SHA1 Fingerprint: {mCert.GetCertHashString()}\n{ExportToPem(mCert)}\n");
      }

      return builder.ToString();
    }

    private static string ExportToPem(X509Certificate2 cert)
      => $"-----BEGIN CERTIFICATE-----\n{Convert.ToBase64String(cert.GetRawCertData(), Base64FormattingOptions.InsertLineBreaks)}\n-----END CERTIFICATE-----";

    public static X509Certificate2? GetServerCertificate(Uri        uri,
                                                         GrpcClient optionsGrpcClient)
    {
      var request = (HttpWebRequest)WebRequest.Create(uri);
      request.ServerCertificateValidationCallback = (_,
                                                     _,
                                                     _,
                                                     _) => true;
      if (optionsGrpcClient.HasClientCertificate)
      {
        request.ClientCertificates.Add(GetCertificate(optionsGrpcClient));
      }

      var response = (HttpWebResponse)request.GetResponse();
      response.Close();
      return request.ServicePoint.Certificate == null
               ? null
               : new X509Certificate2(request.ServicePoint.Certificate.GetRawCertData(),
                                      "",
                                      X509KeyStorageFlags.Exportable);
    }

    public static string? GetOverrideTargetName(GrpcClient        optionsGrpcClient,
                                                X509Certificate2? serverCert)
    {
      if (!optionsGrpcClient.AllowUnsafeConnection)
      {
        return null;
      }

      return optionsGrpcClient.OverrideTargetName != GrpcClient.OverrideTargetNameAutomatic
               ? optionsGrpcClient.OverrideTargetName
               : serverCert?.GetNameInfo(X509NameType.SimpleName,
                                         false);
    }

    /// <summary>
    ///   Creates the GrpcChannel for .Net Framework.
    /// </summary>
    /// <param name="optionsGrpcClient">Options for the creation of the channel</param>
    /// <returns>
    ///   The initialized Channel
    /// </returns>
    private static Channel CreateFrameworkChannel(GrpcClient optionsGrpcClient)
    {
      Environment.SetEnvironmentVariable("GRPC_DNS_RESOLVER",
                                         "native");
      var uri = new Uri(optionsGrpcClient.Endpoint!);

      if (uri.Scheme != Uri.UriSchemeHttps)
      {
        return new Channel(uri.Host,
                           uri.Port,
                           ChannelCredentials.Insecure);
      }

      if (optionsGrpcClient.AllowUnsafeConnection)
      {
        var serverCert = GetServerCertificate(uri,
                                              optionsGrpcClient);
        var credentials = new SslCredentials(serverCert == null
                                               ? null
                                               : ExportToPem(serverCert),
                                             optionsGrpcClient.HasClientCertificate
                                               ? GetKeyCertificatePair(optionsGrpcClient)
                                               : null,
                                             _ => true);
        return new Channel(uri.Host,
                           uri.Port,
                           credentials,
                           new List<ChannelOption>
                           {
                             new("grpc.ssl_target_name_override",
                                 GetOverrideTargetName(optionsGrpcClient,
                                                       serverCert)),
                           });
      }

      var ca = string.IsNullOrEmpty(optionsGrpcClient.CaCert)
                 ? GetRootCertificates()
                 : File.ReadAllText(optionsGrpcClient.CaCert);
      var certKeyPair = optionsGrpcClient.HasClientCertificate
                          ? GetKeyCertificatePair(optionsGrpcClient)
                          : null;

      return new Channel(uri.Host,
                         uri.Port,
                         new SslCredentials(ca,
                                            certKeyPair,
                                            null));
    }

    /// <summary>
    ///   Creates the GrpcChannel
    /// </summary>
    /// <param name="optionsGrpcClient">Options for the creation of the channel</param>
    /// <returns>
    ///   The initialized GrpcChannel
    /// </returns>
    /// <exception cref="InvalidOperationException">Endpoint passed through options is missing</exception>
    public static ChannelBase CreateChannel(GrpcClient optionsGrpcClient)
    {
      if (string.IsNullOrEmpty(optionsGrpcClient.Endpoint))
      {
        throw new InvalidOperationException($"{nameof(optionsGrpcClient.Endpoint)} should not be null or empty");
      }

      if (RuntimeInformation.FrameworkDescription.StartsWith(".NET Framework"))
      {
        // .NET Framework cannot use Grpc.Net.Client.GrpcChannel as it doesn't support Http2 with this framework
        return CreateFrameworkChannel(optionsGrpcClient);
      }

      if (!string.IsNullOrWhiteSpace(optionsGrpcClient.CaCert) && !optionsGrpcClient.AllowUnsafeConnection)
      {
        /* You cannot give a root certificate directly using the C# implementation, thus you have to either :
         - Add the CA to the trusted root store
         - Somehow validate the certificate with a custom root, but it's difficult :
             - https://stackoverflow.com/questions/13103295/bouncy-castle-this-certificate-has-an-invalid-digital-signature
             - https://www.meziantou.net/custom-certificate-validation-in-dotnet.htm
         The issue being that the server certificate is considered to not have a valid signature, and I'm not sure how to handle it
        */
        Console.Error.WriteLine("WARNING : Using gRPC Core (deprecated) implementation because CaCert is specified. Please install the CA certificate and unset the option to use the C# implementation");
        return CreateFrameworkChannel(optionsGrpcClient);
      }

      var uri = new Uri(optionsGrpcClient.Endpoint!);

      var credentials = uri.Scheme == Uri.UriSchemeHttps
                          ? ChannelCredentials.SecureSsl
                          : ChannelCredentials.Insecure;

      var httpHandler = new HttpClientHandler();

      if (optionsGrpcClient.AllowUnsafeConnection)
      {
        httpHandler.ServerCertificateCustomValidationCallback = (_,
                                                                 _,
                                                                 _,
                                                                 _) => true;
        AppContext.SetSwitch("System.Net.Http.SocketsHttpHandler.Http2UnencryptedSupport",
                             true);
      }

      if (uri.Scheme == Uri.UriSchemeHttps)
      {
        if (optionsGrpcClient.HasClientCertificate)
        {
          httpHandler.ClientCertificates.Add(GetCertificate(optionsGrpcClient));
        }

        try
        {
          // try TLS1.3
          ServicePointManager.SecurityProtocol |= (SecurityProtocolType)12288 | SecurityProtocolType.Tls12 | SecurityProtocolType.Tls11 | SecurityProtocolType.Tls;
          httpHandler.SslProtocols             =  (SslProtocols)12288         | SslProtocols.Tls12         | SslProtocols.Tls11         | SslProtocols.Tls;
        }
        catch (NotSupportedException)
        {
          ServicePointManager.SecurityProtocol |= SecurityProtocolType.Tls12 | SecurityProtocolType.Tls11 | SecurityProtocolType.Tls;
          httpHandler.SslProtocols             =  SslProtocols.Tls12         | SslProtocols.Tls11         | SslProtocols.Tls;
        }
      }

      var channelOptions = new GrpcChannelOptions
                           {
                             Credentials = credentials,
                             HttpHandler = httpHandler,
                           };
      return GrpcChannel.ForAddress(optionsGrpcClient.Endpoint!,
                                    channelOptions);
    }

    public static X509Certificate2 GetCertificate(GrpcClient optionsGrpcClient)
    {
      if (!string.IsNullOrEmpty(optionsGrpcClient.CertP12))
      {
        return new X509Certificate2(optionsGrpcClient.CertP12);
      }

      if (string.IsNullOrEmpty(optionsGrpcClient.CertPem) || string.IsNullOrEmpty(optionsGrpcClient.KeyPem))
      {
        throw new InvalidOperationException("Cannot find requested certificate from options");
      }

      X509Certificate cert;
      using (var reader = new FileStream(optionsGrpcClient.CertPem,
                                         FileMode.Open,
                                         FileAccess.Read))
      {
        cert = new X509CertificateParser().ReadCertificate(reader);
      }

      var store = new Pkcs12StoreBuilder().Build();
      using (var reader = new StreamReader(optionsGrpcClient.KeyPem,
                                           Encoding.UTF8))
      {
        var pemReader = new PemReader(reader);
        var keyPair   = pemReader.ReadObject() as AsymmetricCipherKeyPair ?? throw new KeyException("Key could not be retrieved from file");
        store.SetKeyEntry("alias",
                          new AsymmetricKeyEntry(keyPair.Private),
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

    public static KeyCertificatePair GetKeyCertificatePair(GrpcClient optionsGrpcClient)
    {
      if (!string.IsNullOrEmpty(optionsGrpcClient.CertP12))
      {
        var cert = new X509Certificate2(optionsGrpcClient.CertP12,
                                        "",
                                        X509KeyStorageFlags.Exportable);

        if (cert.GetRSAPrivateKey() is not RSA rsaKey)
        {
          throw new
            CryptographicException("Only certificate with RSA key in P12 format is supported in this version. Please use CertPem and KeyPem for other key algorithms.");
        }

        var memoryStream = new MemoryStream();
        using (var streamWriter = new StreamWriter(memoryStream))
        {
          var pemWriter = new PemWriter(streamWriter);
          pemWriter.WriteObject(DotNetUtilities.GetRsaKeyPair(rsaKey)
                                               .Private);
        }

        var keyPem = Encoding.ASCII.GetString(memoryStream.GetBuffer())
                             .Trim();
        memoryStream.Close();

        return new KeyCertificatePair(ExportToPem(cert),
                                      keyPem);
      }

      if (string.IsNullOrEmpty(optionsGrpcClient.CertPem) || string.IsNullOrEmpty(optionsGrpcClient.KeyPem))
      {
        throw new InvalidOperationException("Cannot find requested certificate from options");
      }

      return new KeyCertificatePair(File.ReadAllText(optionsGrpcClient.CertPem),
                                    File.ReadAllText(optionsGrpcClient.KeyPem));
    }
  }
}
