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

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Threading;

using JetBrains.Annotations;

using MethodDecorator.Fody.Interfaces;

namespace ArmoniK.Api.Mock;

public static class CountingService
{
  /// <summary>
  ///   Dictionary with all counts of all counting services.
  ///   `StrongBox` is required to get a `ref` on the value, and cal `Interlocked.increment` on it.
  /// </summary>
  internal static readonly Dictionary<string, Dictionary<string, StrongBox<long>>> Counts;

  /// <summary>
  ///   List of all counting services
  /// </summary>
  private static readonly List<Type> Services;

  static CountingService()
  {
    Services = AppDomain.CurrentDomain.GetAssemblies()
                        // Get all types from assemblies
                        .SelectMany(s =>
                                    {
                                      try
                                      {
                                        return s.GetTypes();
                                      }
                                      catch (ReflectionTypeLoadException e)
                                      {
                                        return e.Types.Where(t => t is not null);
                                      }
                                      catch
                                      {
                                        return Array.Empty<Type>();
                                      }
                                    })
                        // Keep only types that have the [Counting] attribute
                        .Where(type => type is not null && type.GetCustomAttributes<CountingAttribute>()
                                                               .Any())
                        .Select(type => type!)
                        .ToList();

    Counts = Services
      // Create a dictionary for all the types to record method calling counts
      .ToDictionary(type => type.Name,
                    type => type.GetMethods()
                                // Get all methods that have the [Count] attribute
                                .Where(method => method.GetCustomAttributes<CountAttribute>()
                                                       .Any())
                                .ToDictionary(method => method.Name,
                                              _ => new StrongBox<long>(0)));
  }

  /// <summary>
  ///   Get a list of all counting services (ie: types that are marked with [Counting] attribute)
  /// </summary>
  /// <returns>Enumeration of counting services</returns>
  public static IEnumerable<Type> GetServices()
    => Services;

  /// <summary>
  ///   Get the counters for all recorded types and methods.
  /// </summary>
  /// <param name="exclude">Methods to exclude from the count</param>
  /// <returns>Counters</returns>
  public static Dictionary<string, Dictionary<string, long>> GetCounters(IDictionary<string, HashSet<string>>? exclude = null)
    => Counts.ToDictionary(kvType => kvType.Key,
                           kvType =>
                           {
                             var              typeName      = kvType.Key;
                             HashSet<string>? methodExclude = null;
                             exclude?.TryGetValue(typeName,
                                                  out methodExclude);
                             methodExclude ??= new HashSet<string>();

                             return kvType.Value.Where(kv => !methodExclude.Contains(kv.Key))
                                          .ToDictionary(kv => kv.Key,
                                                        kv => kv.Value.Value);
                           });

  /// <summary>
  ///   Reset all the counters
  /// </summary>
  public static void ResetCounters()
  {
    foreach (var counts in Counts)
    {
      foreach (var count in counts.Value)
      {
        count.Value.Value = 0;
      }
    }
  }
}

/// <summary>
///   Mark a class as being a counting service
/// </summary>
[AttributeUsage(AttributeTargets.Class)]
[UsedImplicitly]
public class CountingAttribute : Attribute
{
}

/// <summary>
///   Mark a method to count the number of calls made to this method.
/// </summary>
[AttributeUsage(AttributeTargets.Method)]
[UsedImplicitly]
public class CountAttribute : Attribute, IMethodDecorator
{
  public void Init(object     instance,
                   MethodBase method,
                   object[]   args)
  {
    _ = args;
    Interlocked.Increment(ref CountingService.Counts[instance.GetType()
                                                             .Name][method.Name]
                                             .Value);
  }

  public void OnException(Exception exception)
    => _ = exception;

  public void OnEntry()
  {
  }

  public void OnExit()
  {
  }
}
