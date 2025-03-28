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

using ArmoniK.Api.Common.Channel.Utils;
using ArmoniK.Api.Common.Options;
using ArmoniK.Api.Worker.Utils;
using ArmoniK.Api.Worker.Worker;

using JetBrains.Annotations;

using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
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
    var collection = new List<KeyValuePair<string, string>>
                     {
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.WorkerChannel)}:{nameof(ComputePlane.WorkerChannel.Address)}",
                           "/tmp/worker.sock"),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.WorkerChannel)}:{nameof(ComputePlane.WorkerChannel.SocketType)}",
                           GrpcSocketType.UnixDomainSocket.ToString()),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.AgentChannel)}:{nameof(ComputePlane.AgentChannel.Address)}",
                           "/tmp/agent.sock"),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.AgentChannel)}:{nameof(ComputePlane.AgentChannel.SocketType)}",
                           GrpcSocketType.UnixDomainSocket.ToString()),
                     };

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
  public Task BuildServerConfigurator()
  {
    var collection = new List<KeyValuePair<string, string>>
                     {
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.WorkerChannel)}:{nameof(ComputePlane.WorkerChannel.Address)}",
                           "/tmp/worker.sock"),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.WorkerChannel)}:{nameof(ComputePlane.WorkerChannel.SocketType)}",
                           GrpcSocketType.UnixDomainSocket.ToString()),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.AgentChannel)}:{nameof(ComputePlane.AgentChannel.Address)}",
                           "/tmp/agent.sock"),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.AgentChannel)}:{nameof(ComputePlane.AgentChannel.SocketType)}",
                           GrpcSocketType.UnixDomainSocket.ToString()),
                     };

    var app = WorkerServer.Create<TestService>((_,
                                                configuration) =>
                                               {
                                                 foreach (var pair in collection)
                                                 {
                                                   configuration[pair.Key] = pair.Value;
                                                 }
                                               });
    return Task.CompletedTask;
  }

  [Test]
  public Task BuildServerConfiguratorTcp()
  {
    var collection = new List<KeyValuePair<string, string>>
                     {
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.WorkerChannel)}:{nameof(ComputePlane.WorkerChannel.Address)}",
                           "http://localhost:10667"),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.WorkerChannel)}:{nameof(ComputePlane.WorkerChannel.SocketType)}",
                           GrpcSocketType.Tcp.ToString()),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.WorkerChannel)}:{nameof(ComputePlane.WorkerChannel.KeepAliveTimeOut)}",
                           "MaxValue"),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.WorkerChannel)}:{nameof(ComputePlane.WorkerChannel.KeepAlivePingTimeOut)}",
                           "MaxValue"),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.AgentChannel)}:{nameof(ComputePlane.AgentChannel.Address)}",
                           "http://localhost:10666"),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.AgentChannel)}:{nameof(ComputePlane.AgentChannel.KeepAliveTimeOut)}",
                           TimeSpan.FromSeconds(100)
                                   .ToString()),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.AgentChannel)}:{nameof(ComputePlane.AgentChannel.KeepAlivePingTimeOut)}",
                           TimeSpan.FromSeconds(5)
                                   .ToString()),
                       new($"{nameof(ComputePlane)}:{nameof(ComputePlane.AgentChannel)}:{nameof(ComputePlane.AgentChannel.SocketType)}",
                           GrpcSocketType.Tcp.ToString()),
                     };

    var app = WorkerServer.Create<TestService>((_,
                                                configuration) =>
                                               {
                                                 foreach (var pair in collection)
                                                 {
                                                   configuration[pair.Key] = pair.Value;
                                                 }
                                               });

    var computePlane = app.Services.GetRequiredService<ComputePlane>();

    Assert.AreEqual(computePlane.WorkerChannel.KeepAliveTimeOut,
                    TimeSpan.MaxValue);
    Assert.AreEqual(computePlane.WorkerChannel.KeepAliveTimeOut,
                    TimeSpan.MaxValue);

    Assert.AreEqual(computePlane.AgentChannel.KeepAliveTimeOut,
                    TimeSpan.FromSeconds(100));
    Assert.AreEqual(computePlane.AgentChannel.KeepAlivePingTimeOut,
                    TimeSpan.FromSeconds(5));

    return Task.CompletedTask;
  }

  [Test]
  public Task BuildServerNoArgs()
  {
    Environment.SetEnvironmentVariable($"{nameof(ComputePlane)}__{nameof(ComputePlane.WorkerChannel)}__{nameof(ComputePlane.WorkerChannel.Address)}",
                                       "/tmp/worker.sock");
    Environment.SetEnvironmentVariable($"{nameof(ComputePlane)}__{nameof(ComputePlane.AgentChannel)}__{nameof(ComputePlane.AgentChannel.Address)}",
                                       "/tmp/agent.sock");
    var app = WorkerServer.Create<TestService>();
    return Task.CompletedTask;
  }

  [Test]
  public Task BuildServerAddService()
  {
    Environment.SetEnvironmentVariable($"{nameof(ComputePlane)}__{nameof(ComputePlane.WorkerChannel)}__{nameof(ComputePlane.WorkerChannel.Address)}",
                                       "/tmp/worker.sock");
    Environment.SetEnvironmentVariable($"{nameof(ComputePlane)}__{nameof(ComputePlane.AgentChannel)}__{nameof(ComputePlane.AgentChannel.Address)}",
                                       "/tmp/agent.sock");
    var app = WorkerServer.Create<TestService>(serviceConfigurator: collection => collection.AddSingleton("test"));
    return Task.CompletedTask;
  }
}

public class TestService : WorkerStreamWrapper
{
  public TestService([NotNull] ILoggerFactory      loggerFactory,
                     [NotNull] ComputePlane        computePlane,
                     [NotNull] GrpcChannelProvider provider)
    : base(loggerFactory,
           computePlane,
           provider)
  {
  }
}
