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
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Partitions;
using ArmoniK.Utils;

using Google.Protobuf.WellKnownTypes;

using NUnit.Framework;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class PartitionClientTest
{
  [SetUp]
  public void SetUp()
    => options_ = ConfTest.GetChannelOptions();

  private GrpcClient? options_;

  [Test]
  public void TestGetPartition()
  {
    var before = ConfTest.RpcCalled("Partitions",
                                    "GetPartition")
                         .WaitSync();
    var channel   = GrpcChannelFactory.CreateChannel(options_!);
    var partition = "default";
    var taskOptions = new TaskOptions
                      {
                        MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
                        MaxRetries  = 2,
                        Priority    = 1,
                        PartitionId = partition,
                      };
    var client = new Partitions.PartitionsClient(channel);


    Assert.That(() => client.GetPartition(new GetPartitionRequest
                                          {
                                            Id = taskOptions.PartitionId,
                                          }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Partitions",
                                   "GetPartition")
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }

  [Test]
  public void TestListPartitions()
  {
    var before = ConfTest.RpcCalled("Partitions",
                                    "ListPartitions")
                         .WaitSync();
    var channel = GrpcChannelFactory.CreateChannel(options_!);
    var client  = new Partitions.PartitionsClient(channel);

    Assert.That(() => client.ListPartitions(new ListPartitionsRequest
                                            {
                                              Filters = new Filters(),
                                            }),
                Throws.Nothing);
    var after = ConfTest.RpcCalled("Partitions",
                                   "ListPartitions")
                        .WaitSync();
    Assert.AreEqual(after - before,
                    1);
  }
}
