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
using System.Threading.Tasks;

namespace ArmoniK.Api.Worker.Options;

public static class TaskExt
{
  public static Task<T[]> WhenAll<T>(this IEnumerable<Task<T>> enumerable)
    => Task.WhenAll(enumerable);

  public static Task WhenAll(this IEnumerable<Task> enumerable)
    => Task.WhenAll(enumerable);

  public static async Task<List<T>> ToListAsync<T>(this Task<IEnumerable<T>> enumerableTask)
    => (await enumerableTask.ConfigureAwait(false)).ToList();
}

public static class DisposableExt
{
  public static IAsyncDisposable Merge(this IEnumerable<IAsyncDisposable> disposables)
    => AsyncDisposable.Create(async () => await disposables.Select(async disposable => await disposable.DisposeAsync()
                                                                                                       .ConfigureAwait(false))
                                                           .WhenAll()
                                                           .ConfigureAwait(false));
}

public static class Disposable
{
  public static IDisposable Create(Action action)
    => new DisposableImpl(action);

  private class DisposableImpl : IDisposable
  {
    private readonly Action action_;

    public DisposableImpl(Action action)
      => action_ = action;

    /// <inheritdoc />
    public void Dispose()
      => action_();
  }
}

public static class AsyncDisposable
{
  public static IAsyncDisposable Create(Func<ValueTask> action)
    => new AsyncDisposableImpl(action);

  private class AsyncDisposableImpl : IAsyncDisposable
  {
    private readonly Func<ValueTask> action_;

    public AsyncDisposableImpl(Func<ValueTask> action)
      => action_ = action;

    /// <inheritdoc />
    public ValueTask DisposeAsync()
      => action_();
  }
}
