// This file is part of the ArmoniK project
// 
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

using System;

using Microsoft.Extensions.Configuration;

namespace ArmoniK.Api.Common.Utils;

/// <summary>
///   Extends the functionality of the <see cref="IConfiguration" />
/// </summary>
public static class ConfigurationExt
{
  /// <summary>
  ///   Configure an object with the given configuration.
  /// </summary>
  /// <typeparam name="T">Type of the options class</typeparam>
  /// <param name="configuration">Configurations used to populate the class</param>
  /// <param name="key">Path to the Object in the configuration</param>
  /// <returns>
  ///   The initialized object
  /// </returns>
  /// <exception cref="InvalidOperationException">the <paramref name="key" /> is not found in the configurations.</exception>
  public static T GetRequiredValue<T>(this IConfiguration configuration,
                                      string              key)
    => configuration.GetRequiredSection(key)
                    .Get<T>() ?? throw new InvalidOperationException($"{key} not found");

  /// <summary>
  ///   Retrieves a <see cref="TimeSpan" /> from the configuration, or returns the provided default value if not found or
  ///   invalid.
  ///   If the configuration contains "MaxValue", it will return <see cref="TimeSpan.MaxValue" />.
  /// </summary>
  /// <param name="configuration">The <see cref="IConfiguration" /> instance from which to retrieve the value.</param>
  /// <param name="key">The key of the configuration value to retrieve.</param>
  /// <param name="defaultValue">The default value to return if the key is not found or the value is invalid.</param>
  /// <returns>
  ///   A <see cref="TimeSpan" /> representing the configuration value, or <paramref name="defaultValue" /> if invalid
  ///   or missing.
  /// </returns>
  /// <exception cref="FormatException">Thrown if the value is not a valid <see cref="TimeSpan" /> or "MaxValue".</exception>
  public static TimeSpan GetTimeSpanOrDefault(this IConfiguration configuration,
                                              string              key,
                                              TimeSpan            defaultValue)
  {
    try
    {
      return configuration.GetRequiredValue<TimeSpan>(key);
    }
    catch (Exception)
    {
      var value = configuration.GetValue<string>(key);
      if (string.IsNullOrEmpty(value))
      {
        return defaultValue;
      }

      if (value!.Equals("MaxValue"))
      {
        return TimeSpan.MaxValue;
      }

      throw new FormatException($"'{key}' must be a valid TimeSpan or 'MaxValue'.");
    }
  }
}
