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

using ArmoniK.Api.gRPC.V1.Applications;

using Grpc.Core;

namespace ArmoniK.Api.Mock.Services;

public class ApplicationsService : Applications.ApplicationsBase, ICountingService
{
  private CallCount calls_ = new();

  /// <inheritdocs />
  public ICounter GetCounter()
    => calls_;

  /// <inheritdocs />
  public override Task<ListApplicationsResponse> ListApplications(ListApplicationsRequest request,
                                                                  ServerCallContext       context)
  {
    Interlocked.Add(ref calls_.ListApplications,
                    1);
    return Task.FromResult(new ListApplicationsResponse
                           {
                             Page     = request.Page,
                             PageSize = request.PageSize,
                             Total    = 0,
                           });
  }

  /// <inheritdocs />
  public override Task<CountTasksByStatusResponse> CountTasksByStatus(CountTasksByStatusRequest request,
                                                                      ServerCallContext         context)
  {
    Interlocked.Add(ref calls_.CountTasksByStatus,
                    1);
    return Task.FromResult(new CountTasksByStatusResponse());
  }

  private struct CallCount : ICounter
  {
    public int ListApplications   = 0;
    public int CountTasksByStatus = 0;

    public CallCount()
    {
    }
  }
}
