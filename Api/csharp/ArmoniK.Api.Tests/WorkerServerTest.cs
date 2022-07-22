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
using System.Threading.Tasks;

using ArmoniK.Api.Worker.Utils;
using ArmoniK.Api.Worker.Worker;

using JetBrains.Annotations;

using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;

using NUnit.Framework;

namespace ArmoniK.Api.Worker.Tests;

[TestFixture]
public class WorkerServerTest
{
  [SetUp]
  public void SetUp()
  {
  }

  [TearDown]
  public virtual void TearDown()
  {
  }

  [Test]
  public Task BuildServer()
  {
    var collection = new List<KeyValuePair<string, string>>();

    collection.Add(new KeyValuePair<string, string>("ComputePlan:WorkerChannel:Address",
                                                    "/tmp/worker.sock"));
    collection.Add(new KeyValuePair<string, string>("ComputePlan:WorkerChannel:SocketType",
                                                    "unixdomainsocket"));

    collection.Add(new KeyValuePair<string, string>("ComputePlan:AgentChannel:Address",
                                                    "/tmp/agent.sock"));
    collection.Add(new KeyValuePair<string, string>("ComputePlan:AgentChannel:SocketType",
                                                    "unixdomainsocket"));

    var configuration = new ConfigurationBuilder().AddInMemoryCollection(collection)
                                                  .Build();

    foreach (var pair in configuration.AsEnumerable())
    {
      Console.WriteLine(pair);
    }

    var app = WorkerServer.Create<TestService>(configuration);
    return Task.CompletedTask;
  }

  [Test]
  public Task BuildServerNoArgs()
  {
    System.Environment.SetEnvironmentVariable("ComputePlan__WorkerChannel__Address",
                                              "/tmp/worker.sock");
    var app = WorkerServer.Create<TestService>();
    return Task.CompletedTask;
  }
}

public class TestService : WorkerStreamWrapper
{
  public TestService([NotNull] ILoggerFactory      loggerFactory,
                     [NotNull] GrpcChannelProvider provider)
    : base(loggerFactory,
           provider)
  {
  }
}
