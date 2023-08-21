// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2023.All rights reserved.
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

using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1.Partitions;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class Partitions : gRPC.V1.Partitions.Partitions.PartitionsBase
{
  /// <inheritdocs />
  [Count]
  public override Task<GetPartitionResponse> GetPartition(GetPartitionRequest request,
                                                          ServerCallContext   context)
    => Task.FromResult(new GetPartitionResponse
                       {
                         Partition = new PartitionRaw
                                     {
                                       Id                   = "partition-id",
                                       Priority             = 1,
                                       PodMax               = 1,
                                       PodReserved          = 1,
                                       PreemptionPercentage = 0,
                                     },
                       });


  /// <inheritdocs />
  [Count]
  public override Task<ListPartitionsResponse> ListPartitions(ListPartitionsRequest request,
                                                              ServerCallContext     context)
    => Task.FromResult(new ListPartitionsResponse
                       {
                         PageSize = request.PageSize,
                         Page     = 0,
                         Total    = 0,
                       });
}
