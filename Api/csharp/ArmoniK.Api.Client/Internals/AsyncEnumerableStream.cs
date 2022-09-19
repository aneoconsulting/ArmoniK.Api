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
  public class AsyncEnumerableStream : Stream, IAsyncDisposable
  {
    private readonly IAsyncEnumerator<ReadOnlyMemory<byte>> enumerator_;
    private          int                                    inputOffset_;


    public AsyncEnumerableStream(IAsyncEnumerable<ReadOnlyMemory<byte>> enumerable,
                                 CancellationToken                      cancellationToken = default)
    {
      enumerator_  = enumerable.GetAsyncEnumerator(cancellationToken);
      inputOffset_ = 0;
    }

    public async ValueTask DisposeAsync()
      => await enumerator_.DisposeAsync()
                          .ConfigureAwait(false);

    public override void Flush()
    {
    }

    public override int Read(byte[] buffer,
                             int    offset,
                             int    count)
      => Read(buffer.AsSpan(offset,
                            count));

    public int Read(Span<byte> span)
    {
      if (enumerator_.Current.Length == 0)
      {
        enumerator_.MoveNextAsync()
                   .GetAwaiter()
                   .GetResult();
        inputOffset_ = 0;
      }

      var length = Math.Min(span.Length,
                            enumerator_.Current.Length - inputOffset_);
      enumerator_.Current.Slice(inputOffset_,
                                length)
                 .Span.CopyTo(span);

      inputOffset_ += length;

      return length;
    }

    public override Task<int> ReadAsync(byte[]            buffer,
                                        int               offset,
                                        int               count,
                                        CancellationToken cancellationToken)
      => ReadAsync(buffer.AsMemory(offset,
                                   count),
                   cancellationToken)
        .AsTask();

    public async ValueTask<int> ReadAsync(Memory<byte>      buffer,
                                          CancellationToken cancellationToken = default)
    {
      while (enumerator_.Current.Length == inputOffset_)
      {
        inputOffset_ = 0;
        if (!await enumerator_.MoveNextAsync(cancellationToken)
                              .ConfigureAwait(false))
        {
          return 0;
        }
      }

      var length = Math.Min(buffer.Length,
                            enumerator_.Current.Length - inputOffset_);
      enumerator_.Current.Slice(inputOffset_,
                                length)
                 .CopyTo(buffer);

      inputOffset_ += length;

      return length;
    }


    public override long Seek(long       offset,
                              SeekOrigin origin)
      => throw new NotImplementedException();

    public override void SetLength(long value)
      => throw new NotImplementedException();

    public override void Write(byte[] buffer,
                               int    offset,
                               int    count)
      => throw new NotImplementedException();

    public override bool CanRead
      => true;

    public override bool CanSeek
      => false;

    public override bool CanWrite
      => false;

    public override long Length
      => 0;

    public override long Position
    {
      get => 0;
      set => throw new NotImplementedException();
    }
  }
}
