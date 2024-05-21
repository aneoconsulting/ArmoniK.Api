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
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY, without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

using System;
using System.IO;
using System.Runtime.InteropServices;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;

using Grpc.Core;

namespace ArmoniK.Api.Client.Tests;

public enum ConnectivityKind
{
  Unencrypted,
  TlsInsecure,
  TlsCert,
  TlsStore,
  MTlsInsecure,
  MTlsCert,
  MTlsStore,
}

internal static class ConnectivityKindExt
{
  private static string CertFolder
    => Environment.GetEnvironmentVariable("CertFolder") ?? "../../../../certs";

  internal static bool IsTls(this ConnectivityKind kind)
    => kind switch
       {
         ConnectivityKind.Unencrypted => false,
         _                            => true,
       };

  internal static bool IsInsecure(this ConnectivityKind kind)
    => kind switch
       {
         ConnectivityKind.Unencrypted or ConnectivityKind.TlsInsecure or ConnectivityKind.MTlsInsecure => true,
         _                                                                                             => false,
       };

  internal static bool IsMTls(this ConnectivityKind kind)
    => kind switch
       {
         ConnectivityKind.MTlsInsecure => true,
         ConnectivityKind.MTlsCert     => true,
         ConnectivityKind.MTlsStore    => true,
         _                             => false,
       };

  internal static string? GetCaCertPath(this ConnectivityKind kind)
  {
    switch (kind)
    {
      case ConnectivityKind.TlsCert or ConnectivityKind.MTlsCert:
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows) && !RuntimeInformation.FrameworkDescription.StartsWith(".NET Framework"))
        {
          //Assert.Inconclusive("Library loading bug on Windows");
        }

        return Path.Combine(CertFolder,
                            "server1-ca.pem");
      default:
        return null;
    }
  }

  internal static (string?, string?) GetClientCertPath(this ConnectivityKind kind)
    => kind.IsMTls()
         ? (Path.Combine(CertFolder,
                         "client.pem"), Path.Combine(CertFolder,
                                                     "client.key"))
         : (null, null);

  internal static string GetEndpoint(this ConnectivityKind kind)
    => kind switch
       {
         ConnectivityKind.Unencrypted  => "http://localhost:5000",
         ConnectivityKind.TlsInsecure  => "https://localhost:5001",
         ConnectivityKind.TlsCert      => "https://localhost:5001",
         ConnectivityKind.TlsStore     => "https://localhost:5002",
         ConnectivityKind.MTlsInsecure => "https://localhost:5003",
         ConnectivityKind.MTlsCert     => "https://localhost:5003",
         ConnectivityKind.MTlsStore    => "https://localhost:5004",
         _                             => "http://localhost:5000",
       };

  internal static ChannelBase GetChannel(this ConnectivityKind kind)
  {
    var (certPath, keyPath) = kind.GetClientCertPath();

    return GrpcChannelFactory.CreateChannel(new GrpcClient
                                            {
                                              Endpoint              = kind.GetEndpoint(),
                                              AllowUnsafeConnection = kind.IsInsecure(),
                                              CertPem               = certPath             ?? "",
                                              KeyPem                = keyPath              ?? "",
                                              CaCert                = kind.GetCaCertPath() ?? "",
                                            });
  }
}
