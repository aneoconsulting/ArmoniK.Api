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

using System.Linq;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Results;
using ArmoniK.Utils;

using NUnit.Framework;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class ConnectivityTests
{
  [Test]
  public void ResultsGetServiceConfiguration([Values] ConnectivityKind connectivityKind)
  {
    var channel      = connectivityKind.GetChannel();
    var resultClient = new Results.ResultsClient(channel);

    Assert.That(() => resultClient.GetServiceConfiguration(new Empty()),
                Throws.Nothing);
  }

  [Test]
  public async Task MultipleChannels([Values] ConnectivityKind connectivityKind,
                                     [Values(1,
                                             2,
                                             10,
                                             100)]
                                     int concurrency)
  {
    var channels = await Enumerable.Range(0,
                                          concurrency)
                                   .ParallelSelect(i => Task.FromResult(connectivityKind.GetChannel()))
                                   .ToListAsync()
                                   .ConfigureAwait(false);

    await channels.ParallelForEach(async channel =>
                                   {
                                     var resultClient = new Results.ResultsClient(channel);
                                     await resultClient.GetServiceConfigurationAsync(new Empty())
                                                       .ConfigureAwait(false);
                                   })
                  .ConfigureAwait(false);
  }
}
