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
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;

namespace ArmoniK.Api.Client.Internals
{
  /// <summary>
  ///   IO.Stream that is read from an IAsyncEnumerable
  /// </summary>
  public class AsyncEnumerableStream : Stream, IAsyncDisposable
  {
    private readonly IAsyncEnumerator<ReadOnlyMemory<byte>> enumerator_;
    private          int                                    inputOffset_;

    /// <summary>
    ///   Constructs the stream from an IAsyncEnumerable
    /// </summary>
    /// <param name="enumerable">async enumerable read from the stream</param>
    /// <param name="cancellationToken">cancellation token used to abort the enumeration</param>
    public AsyncEnumerableStream(IAsyncEnumerable<ReadOnlyMemory<byte>> enumerable,
                                 CancellationToken                      cancellationToken = default)
    {
      enumerator_  = enumerable.GetAsyncEnumerator(cancellationToken);
      inputOffset_ = 0;
    }

    /// <inheritdoc />
    public override bool CanRead
      => true;

    /// <inheritdoc />
    public override bool CanSeek
      => false;

    /// <inheritdoc />
    public override bool CanWrite
      => false;

    /// <inheritdoc />
    public override long Length
      => 0;

    /// <inheritdoc />
    public override long Position
    {
      get => 0;
      set => throw new NotImplementedException();
    }

    /// <inheritdoc />
    public async ValueTask DisposeAsync()
      => await enumerator_.DisposeAsync()
                          .ConfigureAwait(false);

    /// <inheritdoc />
    public override void Flush()
    {
    }

    /// <inheritdoc />
    public override int Read(byte[] buffer,
                             int    offset,
                             int    count)
      => Read(buffer.AsSpan(offset,
                            count));

    /// <summary>
    ///   Reads a sequence of bytes from the current stream and advances the position within the stream by the number of
    ///   bytes read.
    /// </summary>
    /// <param name="span">
    ///   An array of bytes. When this method returns, the buffer contains the specified byte array with the
    ///   values replaced by the bytes read from the current source.
    /// </param>
    /// <returns>
    ///   The total number of bytes read into the buffer. This can be less than the number of bytes requested if that
    ///   many bytes are not currently available, or zero (0) if the end of the stream has been reached.
    /// </returns>
    /// <exception cref="T:System.ArgumentNullException"><paramref name="span">buffer</paramref> is null.</exception>
    /// <exception cref="T:System.IO.IOException">An I/O error occurs.</exception>
    /// <exception cref="T:System.ObjectDisposedException">Methods were called after the stream was closed.</exception>
    public int Read(Span<byte> span)
    {
      // More data is needed
      while (enumerator_.Current.Length == inputOffset_)
      {
        inputOffset_ = 0;
        if (!enumerator_.MoveNextAsync()
                        .GetAwaiter()
                        .GetResult())
        {
          return 0;
        }
      }

      // Write only what is asked for
      var length = Math.Min(span.Length,
                            enumerator_.Current.Length - inputOffset_);
      enumerator_.Current.Slice(inputOffset_,
                                length)
                 .Span.CopyTo(span);

      inputOffset_ += length;

      return length;
    }

    /// <inheritdoc />
    public override Task<int> ReadAsync(byte[]            buffer,
                                        int               offset,
                                        int               count,
                                        CancellationToken cancellationToken)
      => ReadAsync(buffer.AsMemory(offset,
                                   count),
                   cancellationToken)
        .AsTask();


    /// <summary>
    ///   Asynchronously reads a sequence of bytes from the current stream, advances the position within the stream by
    ///   the number of bytes read, and monitors cancellation requests.
    /// </summary>
    /// <param name="buffer">The buffer to write the data into.</param>
    /// <param name="cancellationToken">
    ///   The token to monitor for cancellation requests. The default value is
    ///   <see cref="P:System.Threading.CancellationToken.None"></see>.
    /// </param>
    /// <returns>
    ///   A task that represents the asynchronous read operation. The value of the
    ///   <paramref name="TResult">TResult</paramref> parameter contains the total number of bytes read into the buffer. The
    ///   result value can be less than the number of bytes requested if the number of bytes currently available is less than
    ///   the requested number, or it can be 0 (zero) if the end of the stream has been reached.
    /// </returns>
    /// <exception cref="T:System.ArgumentNullException"><paramref name="buffer">buffer</paramref> is null.</exception>
    /// <exception cref="T:System.ObjectDisposedException">The stream has been disposed.</exception>
    /// <exception cref="T:System.InvalidOperationException">The stream is currently in use by a previous read operation.</exception>
    public async ValueTask<int> ReadAsync(Memory<byte>      buffer,
                                          CancellationToken cancellationToken = default)
    {
      // More data is needed
      while (enumerator_.Current.Length == inputOffset_)
      {
        inputOffset_ = 0;
        if (!await enumerator_.MoveNextAsync(cancellationToken)
                              .ConfigureAwait(false))
        {
          return 0;
        }
      }

      // Write only what is asked for
      var length = Math.Min(buffer.Length,
                            enumerator_.Current.Length - inputOffset_);
      enumerator_.Current.Slice(inputOffset_,
                                length)
                 .CopyTo(buffer);

      inputOffset_ += length;

      return length;
    }

    /// <inheritdoc />
    public override long Seek(long       offset,
                              SeekOrigin origin)
      => throw new NotImplementedException();

    /// <inheritdoc />
    public override void SetLength(long value)
      => throw new NotImplementedException();

    /// <inheritdoc />
    public override void Write(byte[] buffer,
                               int    offset,
                               int    count)
      => throw new NotImplementedException();
  }
}
