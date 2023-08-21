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

using ArmoniK.Api.gRPC.V1.Applications;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class Applications : gRPC.V1.Applications.Applications.ApplicationsBase
{
  /// <inheritdocs />
  [Count]
  public override Task<ListApplicationsResponse> ListApplications(ListApplicationsRequest request,
                                                                  ServerCallContext       context)
    => Task.FromResult(new ListApplicationsResponse
                       {
                         Page     = request.Page,
                         PageSize = request.PageSize,
                         Total    = 0,
                       });

  /// <inheritdocs />
  [Count]
  public override Task<CountTasksByStatusResponse> CountTasksByStatus(CountTasksByStatusRequest request,
                                                                      ServerCallContext         context)
    => Task.FromResult(new CountTasksByStatusResponse());
}
