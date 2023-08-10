// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2023.All rights reserved.
//   W.Kirschenmann   <wkirschenmann@aneo.fr>
//   J.Gurhem         <jgurhem@aneo.fr>
//   D.Dubuc          <ddubuc@aneo.fr>
//   L.Ziane Khodja   <lzianekhodja@aneo.fr>
//   F.Lemaitre       <flemaitre@aneo.fr>
//   S.Djebbar        <sdjebbar@aneo.fr>
//   J.Fonseca        <jfonseca@aneo.fr>
//
// This program is free software:you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY, without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.If not, see <http://www.gnu.org/licenses/>.

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Sessions;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

public class SessionsService : Sessions.SessionsBase, ICountingService
{
  private static readonly SessionRaw MockSession = new()
                                                   {
                                                     SessionId = "session-id",
                                                     Status    = SessionStatus.Cancelled,
                                                   };

  private CallCount calls_ = new();

  /// <inheritdocs />
  public ICounter GetCounter()
    => calls_;

  /// <inheritdoc />
  public override Task<CancelSessionResponse> CancelSession(CancelSessionRequest request,
                                                            ServerCallContext    context)
  {
    Interlocked.Add(ref calls_.CancelSession,
                    1);
    return Task.FromResult(new CancelSessionResponse
                           {
                             Session = MockSession,
                           });
  }

  /// <inheritdoc />
  public override Task<GetSessionResponse> GetSession(GetSessionRequest request,
                                                      ServerCallContext context)
  {
    Interlocked.Add(ref calls_.GetSession,
                    1);
    return Task.FromResult(new GetSessionResponse
                           {
                             Session = MockSession,
                           });
  }

  /// <inheritdoc />
  public override Task<ListSessionsResponse> ListSessions(ListSessionsRequest request,
                                                          ServerCallContext   context)
  {
    Interlocked.Add(ref calls_.ListSessions,
                    1);
    return Task.FromResult(new ListSessionsResponse
                           {
                             Page     = 0,
                             PageSize = request.PageSize,
                             Total    = 0,
                           });
  }

  /// <inheritdoc />
  public override Task<CountTasksByStatusResponse> CountTasksByStatus(CountTasksByStatusRequest request,
                                                                      ServerCallContext         context)
  {
    Interlocked.Add(ref calls_.CountTasksByStatus,
                    1);
    return Task.FromResult(new CountTasksByStatusResponse());
  }

  private struct CallCount : ICounter
  {
    public int CancelSession      = 0;
    public int GetSession         = 0;
    public int ListSessions       = 0;
    public int CountTasksByStatus = 0;

    public CallCount()
    {
    }
  }
}
