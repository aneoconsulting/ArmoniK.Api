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

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1.HealthChecks;

using NUnit.Framework;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class HealthCheckTest
{
  [SetUp]
  public void SetUp()
    => options_ = ConfTest.GetChannelOptions();

  private GrpcClient? options_;

  [Test]
  public void TestHealthCheck()
  {
    var before = ConfTest.RpcCalled("HealthChecks",
                                    "CheckHealth")
                         .GetAwaiter()
                         .GetResult();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new HealthChecksService.HealthChecksServiceClient(channel);
    Assert.That(() => client.CheckHealth(new CheckHealthRequest()),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("HealthChecks",
                                   "CheckHealth")
                        .GetAwaiter()
                        .GetResult();
    Assert.AreEqual(after - before,
                    1);
  }
}
