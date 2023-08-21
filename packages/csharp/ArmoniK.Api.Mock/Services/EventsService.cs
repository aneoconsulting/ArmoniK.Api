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

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Events;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class EventsService : Events.EventsBase
{
  /// <inheritdocs />
  [Count]
  public override async Task GetEvents(EventSubscriptionRequest                       request,
                                       IServerStreamWriter<EventSubscriptionResponse> responseStream,
                                       ServerCallContext                              context)
    => await responseStream.WriteAsync(new EventSubscriptionResponse
                                       {
                                         SessionId = "session-id",
                                         NewResult = new EventSubscriptionResponse.Types.NewResult
                                                     {
                                                       ResultId = "result-id",
                                                       OwnerId  = "owner-id",
                                                       Status   = ResultStatus.Created,
                                                     },
                                       })
                           .ConfigureAwait(false);
}
