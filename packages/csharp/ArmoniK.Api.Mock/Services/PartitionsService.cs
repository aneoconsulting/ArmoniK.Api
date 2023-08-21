// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2023. All rights reserved.
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

using Armonik.Api.Grpc.V1.Partitions;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class PartitionsService : Partitions.PartitionsBase
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
