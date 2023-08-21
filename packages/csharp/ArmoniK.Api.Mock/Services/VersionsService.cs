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

using Armonik.Api.Grpc.V1.Versions;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class VersionsService : Versions.VersionsBase
{
  private static readonly string ApiVersion = typeof(Versions.VersionsBase).Assembly.GetName()
                                                                           .Version!.ToString();

  /// <inheritdocs />
  [Count]
  public override Task<ListVersionsResponse> ListVersions(ListVersionsRequest request,
                                                          ServerCallContext   context)
    => Task.FromResult(new ListVersionsResponse
                       {
                         Core = "Unknown",
                         Api  = ApiVersion,
                       });
}
