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

namespace ArmoniK.Api.Mock;

public interface ICounter
{
}

public static class Counter
{
  public static Dictionary<string, int> ToDict(this ICounter?       counter,
                                               IEnumerable<string>? exclude = null)
  {
    if (counter is null)
    {
      return new Dictionary<string, int>();
    }

    var excludeSet = exclude as HashSet<string> ?? exclude?.ToHashSet() ?? new HashSet<string>();
    return counter.GetType()
                  .GetFields()
                  .Select(field => new KeyValuePair<string, object?>(field.Name,
                                                                     field.GetValue(counter)))
                  .Where(prop => prop.Value is not null && !excludeSet.Contains(prop.Key))
                  .ToDictionary(prop => prop.Key,
                                prop => (int)(prop.Value ?? 0));
  }
}
