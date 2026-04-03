// Copyright (C) ANEO, 2021-2026. All rights reserved.
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

using ArmoniK.Api.Common.Utils;

using JetBrains.Annotations;

using Microsoft.Extensions.Configuration;

namespace ArmoniK.Api.Common.Options;

/// <summary>
///   Extension methods for <see cref="GrpcChannel" />.
/// </summary>
[PublicAPI]
public static class GrpcChannelExt
{
  /// <summary>
  ///   Returns a copy of the <see cref="GrpcChannel" /> with its <see cref="GrpcChannel.RetryPolicy" />
  ///   populated from the given <see cref="IConfigurationSection" />.
  ///   Any values not present in the configuration retain the defaults defined on <see cref="GrpcChannelRetryPolicy" />.
  /// </summary>
  /// <param name="channel">The source channel options.</param>
  /// <param name="configuration">The configuration section that contains the channel settings.</param>
  /// <returns>A new <see cref="GrpcChannel" /> instance with the retry policy bound from configuration.</returns>
  public static GrpcChannel BuildWithRetryPolicy(this GrpcChannel      channel,
                                      IConfigurationSection configuration)
  {
    var retrySection = configuration.GetSection("RetryPolicy");
    var defaults     = new GrpcChannelRetryPolicy();

    var retryPolicy = new GrpcChannelRetryPolicy
                      {
                        MaxAttempts       = retrySection.GetValue("MaxAttempts",       defaults.MaxAttempts),
                        InitialBackoff    = retrySection.GetTimeSpanOrDefault("InitialBackoff",    defaults.InitialBackoff),
                        MaxBackoff        = retrySection.GetTimeSpanOrDefault("MaxBackoff",        defaults.MaxBackoff),
                        BackoffMultiplier = retrySection.GetValue("BackoffMultiplier", defaults.BackoffMultiplier),
                      };

    return channel.WithRetryPolicy(retryPolicy);
  }
}
