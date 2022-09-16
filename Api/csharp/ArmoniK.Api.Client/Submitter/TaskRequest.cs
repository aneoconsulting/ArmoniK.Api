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

using ArmoniK.Api.Client.Internals;

namespace ArmoniK.Api.Client.Submitter
{
  public struct TaskRequest
  {
    private TaskRequest(IEnumerable<string> expectedOutputKeys,
                        IEnumerable<string> dataDependencies,
                        IPayload            payload)
    {
      ExpectedOutputKeys = expectedOutputKeys;
      DataDependencies   = dataDependencies;
      Payload            = payload;
    }

    public TaskRequest(IEnumerable<string> expectedOutputKeys,
                       IEnumerable<string> dataDependencies,
                       Stream              stream)
      : this(expectedOutputKeys,
             dataDependencies,
             new StreamPayload(stream))
    {
    }

    public TaskRequest(IEnumerable<string> expectedOutputKeys,
                       IEnumerable<string> dataDependencies,
                       byte[]              bytes)
      : this(expectedOutputKeys,
             dataDependencies,
             new ReadOnlyByteArrayPayload(bytes))
    {
    }

    public TaskRequest(IEnumerable<string>  expectedOutputKeys,
                       IEnumerable<string>  dataDependencies,
                       ReadOnlyMemory<byte> readOnlyMemory)
      : this(expectedOutputKeys,
             dataDependencies,
             new ReadOnlyByteMemoryPayload(readOnlyMemory))
    {
    }

    internal IEnumerable<string> ExpectedOutputKeys { get; }
    internal IEnumerable<string> DataDependencies   { get; }
    internal IPayload            Payload            { get; }
  }
}
