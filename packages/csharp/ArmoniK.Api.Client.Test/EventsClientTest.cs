// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-$CURRENT_YEAR$. All rights reserved.
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

using System;
using System.Linq;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Events;
using ArmoniK.Api.gRPC.V1.Sessions;
using ArmoniK.Api.gRPC.V1.Results;

using Google.Protobuf.WellKnownTypes;

using NUnit.Framework;

using Filters = ArmoniK.Api.gRPC.V1.Results.Filters;
using FilterField = ArmoniK.Api.gRPC.V1.Results.FilterField;
using FiltersAnd = ArmoniK.Api.gRPC.V1.Results.FiltersAnd;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class EventsClientTest
{
  [Test]
  public void TestGetEvents()
  {
    var endpoint = Environment.GetEnvironmentVariable("Grpc__Endpoint");
    var channel = GrpcChannelFactory.CreateChannel(new GrpcClient
    {
      Endpoint = endpoint,
    });
    var partition = "default";
    var taskOptions = new TaskOptions
    {
      MaxDuration = Duration.FromTimeSpan(TimeSpan.FromHours(1)),
      MaxRetries = 2,
      Priority = 1,
      PartitionId = partition,
    };

    var session = new Sessions.SessionsClient(channel).CreateSession(new CreateSessionRequest
    {
      DefaultTaskOption = taskOptions,
      PartitionIds =
                                                                       {
                                                                         partition,
                                                                       },
    });

    var client = new Events.EventsClient(channel);

    var resultId = new Results.ResultsClient(channel).CreateResultsMetaData(new CreateResultsMetaDataRequest
                                                                            {
                                                                              SessionId = session.SessionId,
                                                                              Results =
                                                                              {
                                                                                new CreateResultsMetaDataRequest.Types.ResultCreate
                                                                                {
                                                                                  Name = "Result",
                                                                                }
                                                                              },
                                                                            })
                                                     .Results.Single()
                                                     .ResultId;

    Assert.That(() => client.GetEvents(new EventSubscriptionRequest
                                       {
                                         SessionId = session.SessionId,
                                         ReturnedEvents =
                                         {
                                           EventsEnum.ResultStatusUpdate,
                                           EventsEnum.NewResult,
                                         },
                                         ResultsFilters = new Filters
                                                          {
                                                            Or =
                                                            {
                                                              new FiltersAnd
                                                              {
                                                                And =
                                                                {
                                                                  new FilterField
                                                                  {
                                                                    Field = new ResultField
                                                                            {
                                                                              ResultRawField = new ResultRawField
                                                                                               {
                                                                                                 Field = ResultRawEnumField.ResultId,
                                                                                               },
                                                                            },
                                                                    FilterString = new FilterString
                                                                                   {
                                                                                     Operator = FilterStringOperator.Equal,
                                                                                     Value    = resultId,
                                                                                   },
                                                                  },
                                                                }
                                                              }
                                                            }
                                                          }
                                       }), Throws.Nothing);
  }
}
