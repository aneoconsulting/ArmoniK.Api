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

using JetBrains.Annotations;

namespace ArmoniK.Api.Worker.Options;

[PublicAPI]
public class ComputePlan
{
  public const string SettingSection = nameof(ComputePlan);

  /// <summary>
  /// Channel used by the Agent to send tasks to the Worker
  /// </summary>
  public GrpcChannel WorkerChannel { get; set; } = new GrpcChannel();

  /// <summary>
  /// Channel used by the Worker to send requests to the Agent
  /// </summary>
  public GrpcChannel AgentChannel { get; set; } = new GrpcChannel();

  /// <summary>
  /// Number of messages retrieved from the queue by the Agent
  /// </summary>
  public int MessageBatchSize { get; set; } = 1;
}
