// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-$CURRENT_YEAR$. All rights reserved.
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

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Utils;

using Grpc.Core;
using Grpc.Net.Client.Configuration;

using NUnit.Framework;

[TestFixture]
public class ServiceConfigTest
{

  [Test]
  public void CreateJsonFromServiceConfig()
  {
    var config = new GrpcClient();
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
                                              MaxAttempts       = config.MaxAttempts,
                                              InitialBackoff    = config.InitialBackOff,
                                              MaxBackoff        = config.MaxBackOff,
                                              BackoffMultiplier = config.BackoffMultiplier,
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

    Console.WriteLine(serviceConfig.ToJson());
  }

}
