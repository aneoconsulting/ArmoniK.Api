// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2025.All rights reserved.
// 
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.Common.Channel.Utils;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Worker;

namespace ArmoniK.Api.Mock;

public class WorkerCallService(GrpcChannelProvider provider)
{
  public async Task<HealthCheckReply> HealthCheckRequest()
  {
    var channel      = provider.Get();
    var workerClient = new Worker.WorkerClient(channel);
    var reply        = await workerClient.HealthCheckAsync(new Empty());
    await channel.ShutdownAsync()
                 .WaitAsync(CancellationToken.None)
                 .ConfigureAwait(false);
    return reply;
  }

  public async Task<ProcessReply> ProcessRequest(ProcessRequest request)
  {
    var channel      = provider.Get();
    var workerClient = new Worker.WorkerClient(channel);
    var reply        = await workerClient.ProcessAsync(request);
    await channel.ShutdownAsync()
                 .WaitAsync(CancellationToken.None)
                 .ConfigureAwait(false);
    return reply;
  }
}

public enum ResultsEncoding
{
  Base64,
  Hex,
}

public struct WorkerCallServiceInputModel
{
  public string                     Request;
  public Dictionary<string, string> Results;
  public ResultsEncoding            ResultsEncoding;
}
