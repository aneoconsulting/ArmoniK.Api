// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2025. All rights reserved.
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
  /// Retrieves a <see cref="TimeSpan"/> from the configuration, or returns the provided default value if not found or invalid.
  /// If the configuration contains "MaxValue", it will return <see cref="TimeSpan.MaxValue"/>.
  /// </summary>
  /// <param name="configuration">The <see cref="IConfiguration"/> instance from which to retrieve the value.</param>
  /// <param name="key">The key of the configuration value to retrieve.</param>
  /// <param name="defaultValue">The default value to return if the key is not found or the value is invalid.</param>
  /// <returns>A <see cref="TimeSpan"/> representing the configuration value, or <paramref name="defaultValue"/> if invalid or missing.</returns>
  /// <exception cref="FormatException">Thrown if the value is not a valid <see cref="TimeSpan"/> or "MaxValue".</exception>
  public static TimeSpan GetTimeSpanOrDefault(this IConfiguration configuration,
                                              string              key,
                                              TimeSpan            defaultValue)
  {
    try
    {
      return GetRequiredValue<TimeSpan>(configuration,
                                        key);
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
