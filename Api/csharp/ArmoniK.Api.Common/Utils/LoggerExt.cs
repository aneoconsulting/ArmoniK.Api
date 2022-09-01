// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2022. All rights reserved.
//   W. Kirschenmann   <wkirschenmann@aneo.fr>
//   J. Gurhem         <jgurhem@aneo.fr>
//   D. Dubuc          <ddubuc@aneo.fr>
//   L. Ziane Khodja   <lzianekhodja@aneo.fr>
//   F. Lemaitre       <flemaitre@aneo.fr>
//   S. Djebbar        <sdjebbar@aneo.fr>
//   J. Fonseca        <jfonseca@aneo.fr>
// 
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//         http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;

using JetBrains.Annotations;

using Microsoft.Extensions.Logging;

namespace ArmoniK.Api.Common.Utils;

/// <summary>
///   Convenience methods for logging
/// </summary>
[PublicAPI]
public static class LoggerExt
{
  /// <summary>
  ///   Create a <see cref="IDisposable" /> named logging scope that will start and end.
  ///   It brings the given properties to every logs occurring in the scope
  /// </summary>
  /// <param name="logger">Logger that will produce logs</param>
  /// <param name="name">Name of the scope</param>
  /// <param name="properties">Properties to put in the scope</param>
  /// <returns>
  ///   <see cref="IDisposable" /> object that can be dispose to terminate the scope
  /// </returns>
  public static IDisposable BeginNamedScope(this ILogger                        logger,
                                            string                              name,
                                            params ValueTuple<string, object>[] properties)
  {
    var dictionary = properties.ToDictionary(p => p.Item1,
                                             p => p.Item2);
    dictionary[name + ".Scope"] = Guid.NewGuid();
    return logger.BeginScope(dictionary);
  }

  /// <summary>
  ///   Create a <see cref="IDisposable" /> logging scope that will start and end.
  ///   It brings the given properties to every logs occurring in the scope
  /// </summary>
  /// <param name="logger">Logger that will produce logs</param>
  /// <param name="properties">Properties to put in the scope</param>
  /// <returns>
  ///   <see cref="IDisposable" /> object that can be dispose to terminate the scope
  /// </returns>
  public static IDisposable BeginPropertyScope(this   ILogger                      logger,
                                               params ValueTuple<string, object>[] properties)
  {
    var dictionary = properties.ToDictionary(p => p.Item1,
                                             p => p.Item2);
    return logger.BeginScope(dictionary);
  }


  /// <summary>
  ///   Logs the entering and leaving of a function represented by a scope
  /// </summary>
  /// <param name="logger">Logger that will produce logs</param>
  /// <param name="id">Id of the under laying scope</param>
  /// <param name="level">Level of logs produced</param>
  /// <param name="functionName">Name of the function, found automatically with annotation</param>
  /// <param name="classFilePath">Path to the filename containing the function, found automatically with annotation</param>
  /// <param name="line">Line of the function, found automatically with annotation</param>
  /// <returns>
  ///   <see cref="IDisposable" /> object that can be dispose to terminate the scope
  /// </returns>
  public static IDisposable LogFunction(this ILogger              logger,
                                        string                    id            = "",
                                        LogLevel                  level         = LogLevel.Trace,
                                        [CallerMemberName] string functionName  = "",
                                        [CallerFilePath]   string classFilePath = "",
                                        [CallerLineNumber] int    line          = 0)
  {
    if (!logger.IsEnabled(level))
    {
      return Disposable.Empty;
    }

    var properties = new List<ValueTuple<string, object>>
                     {
                       (nameof(functionName), functionName),
                       (nameof(classFilePath), classFilePath),
                       (nameof(line), line),
                     };
    if (!string.IsNullOrEmpty(id))
    {
      properties.Add(("Id", id));
    }

    var scope = logger.BeginNamedScope($"{classFilePath}.{functionName}",
                                       properties.ToArray());

    try
    {
      logger.Log(level,
                 "Entering {classFilePath}.{functionName} - {Id}",
                 classFilePath,
                 functionName,
                 id);
    }
    finally
    {
      scope.Dispose();
    }

    return Disposable.Create(() =>
                             {
                               using (scope)
                               {
                                 logger.Log(level,
                                            "Leaving {classFilePath}.{functionName} - {Id}",
                                            classFilePath,
                                            functionName,
                                            id);
                               }
                             });
  }
}
