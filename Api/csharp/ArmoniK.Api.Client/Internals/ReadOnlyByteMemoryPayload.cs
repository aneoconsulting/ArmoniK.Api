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
using System.Linq;
using System.Threading;

using ArmoniK.Api.Client.Submitter;

using Google.Protobuf;

namespace ArmoniK.Api.Client.Internals
{
  internal class ReadOnlyByteMemoryPayload : IPayload
  {
    private readonly ReadOnlyMemory<byte> readOnlyMemory_;

    public ReadOnlyByteMemoryPayload(ReadOnlyMemory<byte> readOnlyMemory)
      => readOnlyMemory_ = readOnlyMemory;

    public IAsyncEnumerable<ByteString> ToChunkedByteStringAsync(int               maxChunkSize,
                                                                 CancellationToken cancellationToken = default)
      => ToChunkedByteString(maxChunkSize,
                             cancellationToken)
        .ToAsyncEnumerable();

    private IEnumerable<ByteString> ToChunkedByteString(int               maxChunkSize,
                                                        CancellationToken cancellationToken = default)
    {
      var start = 0;

      while (start < readOnlyMemory_.Length)
      {
        cancellationToken.ThrowIfCancellationRequested();
        var chunkSize = Math.Min(maxChunkSize,
                                 readOnlyMemory_.Length - start);
        yield return UnsafeByteOperations.UnsafeWrap(readOnlyMemory_.Slice(start,
                                                                           chunkSize));
        start += chunkSize;
      }
    }
  }
}
