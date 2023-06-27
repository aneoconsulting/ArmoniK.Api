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

using System.Collections.Generic;
using System.Linq;

using Google.Protobuf.WellKnownTypes;

using Grpc.Net.Client.Configuration;

using Newtonsoft.Json;

namespace ArmoniK.Api.Client.Utils
{
  public static class ServiceConfigExt
  {
    private const string MaxAttemptsPropertyName          = "maxAttempts";
    private const string InitialBackoffPropertyName       = "initialBackoff";
    private const string MaxBackoffPropertyName           = "maxBackoff";
    private const string BackoffMultiplierPropertyName    = "backoffMultiplier";
    private const string RetryableStatusCodesPropertyName = "retryableStatusCodes";
    private const string MethodConfigPropertyName         = "methodConfig";
    private const string MethodNamePropertyName           = "name";
    private const string MethodServicePropertyName        = "service";
    private const string MethodPropertyName               = "method";
    private const string RetryPolicyPropertyName          = "retryPolicy";

    public static string ToJson(this ServiceConfig config)
      => JsonConvert.SerializeObject(config.ToDict());

    public static Dictionary<string, object> ToDict(this ServiceConfig config)
      => new()
         {
           [MethodConfigPropertyName] = config.MethodConfigs.Select(methodConfig => methodConfig.ToDict())
                                              .ToArray(),
         };

    public static Dictionary<string, object> ToDict(this MethodConfig config)
    {
      var dict = new Dictionary<string, object>
                 {
                   [MethodNamePropertyName] = config.Names.Select(methodName => methodName.ToDict())
                                                    .ToArray(),
                 };
      if (config.RetryPolicy is not null)
      {
        dict[RetryPolicyPropertyName] = config.RetryPolicy.ToDict();
      }

      return dict;
    }

    public static Dictionary<string, string> ToDict(this MethodName methodName)
    {
      var dict = new Dictionary<string, string>();
      if (methodName.Service is not null)
      {
        dict[MethodServicePropertyName] = methodName.Service;
      }

      if (methodName.Method is not null)
      {
        dict[MethodPropertyName] = methodName.Method;
      }

      return dict;
    }

    public static Dictionary<string, object> ToDict(this RetryPolicy retryPolicy)
    {
      var dict = new Dictionary<string, object>();
      if (retryPolicy.BackoffMultiplier is not null)
      {
        dict[BackoffMultiplierPropertyName] = retryPolicy.BackoffMultiplier.Value;
      }

      if (retryPolicy.InitialBackoff is not null)
      {
        dict[InitialBackoffPropertyName] = Duration.FromTimeSpan(retryPolicy.InitialBackoff.Value)
                                                   .ToSimpleString();
      }

      if (retryPolicy.MaxAttempts is not null)
      {
        dict[MaxAttemptsPropertyName] = retryPolicy.MaxAttempts.Value;
      }

      if (retryPolicy.MaxBackoff is not null)
      {
        dict[MaxBackoffPropertyName] = Duration.FromTimeSpan(retryPolicy.MaxBackoff.Value)
                                               .ToSimpleString();
      }

      if (retryPolicy.RetryableStatusCodes.Count > 0)
      {
        dict[RetryableStatusCodesPropertyName] = retryPolicy.RetryableStatusCodes.Select(status => status.ToString()
                                                                                                         .ToUpper())
                                                            .ToArray();
      }

      return dict;
    }

    public static string ToSimpleString(this Duration duration)
      => duration.Seconds + (duration.Nanos > 0
                               ? $".{duration.Nanos: D9}s"
                               : "s");
  }
}
