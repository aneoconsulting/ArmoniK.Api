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

using ArmoniK.Api.Common.Channel.Utils;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Worker;

using System.Threading.Tasks;

namespace ArmoniK.Api.Mock
{
  public class WorkerCallService(GrpcChannelProvider provider)
  {
    public async Task<HealthCheckReply> HealthCheckRequest()
    {
      var channel      = provider.Get();
      var workerClient = new Worker.WorkerClient(channel);
      return await workerClient.HealthCheckAsync(new Empty());
    }

    public async Task<ProcessReply> ProcessRequest(ProcessRequest request)
    {
      var channel      = provider.Get();
      var workerClient = new Worker.WorkerClient(channel);
      return await workerClient.ProcessAsync(request);
    }
  }

  public enum ResultsEncoding
  {
    Base64,
    Hex,
  }
  /*
  public struct WorkerCallServiceInputRequestConfigurationModel
  {
    public int DataChunkMaxSize;
  }

  public struct Duration
  {
    public long Seconds;
    public int  Nanos;
  }

  public struct WorkerCallServiceInputRequestTaskOptionsModel
  {
    public string ApplicationName;
    public string ApplicationNamespace;

    public string ApplicationService;

    public string ApplicationVersion;

    public string EngineType;

    public Duration MaxDuration;

    public int MaxRetries;

    public Dictionary<string, string> Options;
    public string                     PartitionId;
    public int                        Priority;

    public TaskOptions ToTaskOptions()
      => new()
         {
           ApplicationName      = ApplicationName,
           ApplicationService   = ApplicationService,
           ApplicationNamespace = ApplicationNamespace,
           ApplicationVersion   = ApplicationVersion,
           EngineType           = EngineType,
           MaxDuration = new Google.Protobuf.WellKnownTypes.Duration
                         {
                           Seconds = MaxDuration.Seconds,
                           Nanos   = MaxDuration.Nanos,
                         },
           MaxRetries = MaxRetries,
           Options =
           {
             Options,
           },
           PartitionId = PartitionId,
           Priority    = Priority,
         };
  }

  public struct WorkerCallServiceInputRequestModel
  {
    public string                                          CommunicationToken;
    public WorkerCallServiceInputRequestConfigurationModel Configuration;
    public List<string>                                    DataDependencies;
    public string                                          DataFolder;
    public List<string>                                    ExpectedOutputKeys;
    public string                                          PayloadId;
    public string                                          SessionId;
    public string                                          TaskId;
    public WorkerCallServiceInputRequestTaskOptionsModel   TaskOptions;

    public ProcessRequest ToProcessRequest()
      => new()
         {
           CommunicationToken = CommunicationToken,
           Configuration = new Configuration
                           {
                             DataChunkMaxSize = Configuration.DataChunkMaxSize
                           },
           DataDependencies =
           {
             DataDependencies,
           },
           DataFolder = DataFolder,
           ExpectedOutputKeys =
           {
             ExpectedOutputKeys,
           },
           PayloadId   = PayloadId,
           SessionId   = SessionId,
           TaskId      = TaskId,
           TaskOptions = TaskOptions.ToTaskOptions(),
         };
  }
  */

  public struct WorkerCallServiceInputModel
  {
    public string                             Request;
    public Dictionary<string, string>         Results;
    public ResultsEncoding                    ResultsEncoding;
  }
}
