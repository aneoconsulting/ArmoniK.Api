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

namespace ArmoniK.Api.Mock;

public interface ICountingService
{
  public ICounter GetCounter();
}

public static class CountingService
{
  public static Dictionary<string, Dictionary<string, int>> GetCounters(this IServiceProvider                 services,
                                                                        IDictionary<string, HashSet<string>>? exclude = null)
    => AppDomain.CurrentDomain.GetAssemblies()
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
                // Keep only concrete classes that implement counter
                .Where(p => typeof(ICountingService).IsAssignableFrom(p) && !p.IsAbstract)
                .Select(type =>
                        {
                          if (type is null || services.GetService(type) is not ICountingService service)
                          {
                            return new KeyValuePair<string, Dictionary<string, int>>("",
                                                                                     new Dictionary<string, int>());
                          }

                          HashSet<string>? counterExclude = null;
                          exclude?.TryGetValue(type.Name,
                                               out counterExclude);

                          return new KeyValuePair<string, Dictionary<string, int>>(type.Name,
                                                                                   service.GetCounter()
                                                                                          .ToDict(counterExclude));
                        })
                .Where(kv => !string.IsNullOrEmpty(kv.Key))
                .ToDictionary(kv => kv.Key,
                              kv => kv.Value);
}
