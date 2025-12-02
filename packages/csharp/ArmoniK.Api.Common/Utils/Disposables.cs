using System;

namespace ArmoniK.Api.Common.Utils;

/// Helper class for disposable's
public static class Disposables
{
  private class NullDisposableInternal : IDisposable
  {
    public readonly static NullDisposableInternal Instance = new();

    public void Dispose() { }
  }


  /// Gets a disposable instance which does nothing
  public static IDisposable NullDisposable => NullDisposableInternal.Instance;
}
