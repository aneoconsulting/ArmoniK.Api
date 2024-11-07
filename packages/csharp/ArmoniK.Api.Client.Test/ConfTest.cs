// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-$CURRENT_YEAR.All rights reserved.
//   W.Kirschenmann   <wkirschenmann@aneo.fr>
//   J.Gurhem         <jgurhem@aneo.fr>
//   D.Dubuc          <ddubuc@aneo.fr>
//   L.Ziane Khodja   <lzianekhodja@aneo.fr>
//   F.Lemaitre       <flemaitre@aneo.fr>
//   S.Djebbar        <sdjebbar@aneo.fr>
//   J.Fonseca        <jfonseca@aneo.fr>
//
// This program is free software:you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY, without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.If not, see <http://www.gnu.org/licenses/>.

using System;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Net.Security;
using System.Runtime.InteropServices;
using System.Security.Cryptography.X509Certificates;
using System.Threading.Tasks;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;

using Microsoft.Extensions.Configuration;

using Newtonsoft.Json.Linq;

using Org.BouncyCastle.X509;

using X509Certificate = Org.BouncyCastle.X509.X509Certificate;

namespace ArmoniK.Api.Client.Tests;

public class ConfTest
{
  public static GrpcClient GetChannelOptions()
  {
    var builder       = new ConfigurationBuilder().AddEnvironmentVariables();
    var configuration = builder.Build();
    var options = configuration.GetRequiredSection(GrpcClient.SettingSection)
                               .Get<GrpcClient>()!;
    if (RuntimeInformation.FrameworkDescription.StartsWith(".NET Framework") || options.HttpMessageHandler.ToLower()
                                                                                       .Contains("web"))
    {
      options!.Endpoint = Environment.GetEnvironmentVariable("Http__Endpoint");
    }

    return options;
  }

  public static async Task<uint> RpcCalled(string service_name,
                                           string rpc_name)
  {
    var options = GetChannelOptions();

    X509Certificate? caCert = null;
    if (!string.IsNullOrWhiteSpace(options.CaCert) && !options.AllowUnsafeConnection)
    {
      if (!File.Exists(options.CaCert))
      {
        throw new FileNotFoundException("Couldn't find specified CA certificate",
                                        options.CaCert);
      }

      var parser = new X509CertificateParser();
      using var stream = File.Open(options.CaCert,
                                   FileMode.Open,
                                   FileAccess.Read,
                                   FileShare.Read);
      caCert = parser.ReadCertificate(stream);
    }

    var clientCert = options.HasClientCertificate
                       ? GrpcChannelFactory.GetCertificate(options)
                       : null;
    var handler = new HttpClientHandler();

    if (clientCert != null)
    {
      handler.ClientCertificates.Add(clientCert!);
    }

    handler.ServerCertificateCustomValidationCallback = (httpRequestMessage,
                                                         cert,
                                                         certChain,
                                                         sslPolicyErrors) =>
                                                        {
                                                          if (!options.AllowUnsafeConnection)
                                                          {
                                                            if (caCert != null)
                                                            {
                                                              certChain.ChainPolicy.ExtraStore.Add(new X509Certificate2(caCert!.GetEncoded()));
                                                              certChain.ChainPolicy.VerificationFlags = X509VerificationFlags.AllowUnknownCertificateAuthority;
                                                              certChain.ChainPolicy.RevocationMode    = X509RevocationMode.NoCheck;
                                                              return certChain.Build(cert);
                                                            }

                                                            return sslPolicyErrors == SslPolicyErrors.None;
                                                          }

                                                          return true;
                                                        };
    var client        = new HttpClient(handler);
    var call_endpoint = Environment.GetEnvironmentVariable("Http__Endpoint") + "/calls.json";
    try
    {
      using var response = await client.GetAsync(call_endpoint);
      response.EnsureSuccessStatusCode();
      var responseBody = response.Content.ReadAsStringAsync()
                                 .Result;
      var jsonResponse = JObject.Parse(responseBody);
      return (uint)jsonResponse[service_name]![rpc_name]!;
    }
    catch (HttpRequestException e)
    {
      Console.WriteLine("Error in HTTP request " + e);
      return 0;
    }
  }
}
