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

using System.Reflection;
using System.Runtime.CompilerServices;

using JetBrains.Annotations;

using MethodDecorator.Fody.Interfaces;

namespace ArmoniK.Api.Mock;

public static class CountingService
{
  /// <summary>
  /// Dictionary with all counts of all counting services
  /// </summary>
  internal static readonly Dictionary<string, Dictionary<string, StrongBox<long>>> Counts;

  static CountingService()
    => Counts = AppDomain.CurrentDomain.GetAssemblies()
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
                         // Create a dictionary for all the types to record method calling counts
                         .ToDictionary(type => type!.Name,
                                       type => type!.GetMethods()
                                                    // Get all methods that have the [Count] attribute
                                                    .Where(method => method.GetCustomAttributes<CountAttribute>()
                                                                           .Any())
                                                    .ToDictionary(method => method.Name,
                                                                  _ => new StrongBox<long>(0)));

  /// <summary>
  /// Get the counters for all recorded types and methods.
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
  /// Reset all the counters
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
/// Mark a class as being a counting service
/// </summary>
[AttributeUsage(AttributeTargets.Class)]
[UsedImplicitly]
public class CountingAttribute : Attribute
{
}

/// <summary>
/// Mark a method to count the number of calls made to this method.
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
