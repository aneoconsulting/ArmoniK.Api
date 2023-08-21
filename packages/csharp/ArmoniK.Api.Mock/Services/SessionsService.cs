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
using ArmoniK.Api.gRPC.V1.Sessions;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

[Counting]
public class SessionsService : Sessions.SessionsBase
{
  private static readonly SessionRaw MockSession = new()
                                                   {
                                                     SessionId = "session-id",
                                                     Status    = SessionStatus.Cancelled,
                                                   };

  /// <inheritdoc />
  [Count]
  public override Task<CancelSessionResponse> CancelSession(CancelSessionRequest request,
                                                            ServerCallContext    context)
    => Task.FromResult(new CancelSessionResponse
                       {
                         Session = MockSession,
                       });

  /// <inheritdoc />
  [Count]
  public override Task<GetSessionResponse> GetSession(GetSessionRequest request,
                                                      ServerCallContext context)
    => Task.FromResult(new GetSessionResponse
                       {
                         Session = MockSession,
                       });

  /// <inheritdoc />
  [Count]
  public override Task<ListSessionsResponse> ListSessions(ListSessionsRequest request,
                                                          ServerCallContext   context)
    => Task.FromResult(new ListSessionsResponse
                       {
                         Page     = 0,
                         PageSize = request.PageSize,
                         Total    = 0,
                       });

  /// <inheritdoc />
  [Count]
  public override Task<CountTasksByStatusResponse> CountTasksByStatus(CountTasksByStatusRequest request,
                                                                      ServerCallContext         context)
    => Task.FromResult(new CountTasksByStatusResponse());
}
