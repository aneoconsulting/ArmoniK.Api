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
using System.Collections;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.Client.Internals;
using ArmoniK.Api.Client.Submitter;

using NUnit.Framework;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class PayloadTest
{
  [SetUp]
  public void SetUp()
  {
  }

  [TearDown]
  public virtual void TearDown()
  {
  }

  public static IEnumerable TestCases(int n)
  {

    var bytes = Encoding.ASCII.GetBytes("test");

    for (var i = 1; i < bytes.Length + n; i++)
    {
      yield return new TestCaseData(new ReadOnlyByteArrayPayload(bytes),
                                    bytes,
                                    i);
      yield return new TestCaseData(new ReadOnlyByteMemoryPayload(bytes),
                                    bytes,
                                    i);
      yield return new TestCaseData(new StreamPayload(new MemoryStream(bytes)),
                                    bytes,
                                    i);
    }
  }


  [Test]
  [TestCaseSource(nameof(TestCases), new object[]{3})]
  public async Task ChunkingShouldSucceed(IPayload payload, byte[] bytes, int maxChunkSize)
  {
    var res = new byte[bytes.Length];
    var idx = 0;
    await foreach (var rm in payload.ToChunkedByteStringAsync(maxChunkSize,
                                                              CancellationToken.None))
    {
      rm.Memory.CopyTo(res.AsMemory(idx,
                                    rm.Length));
      idx += rm.Length;
    }

    Assert.AreEqual(bytes,
                    res);
  }

  [Test]
  [TestCaseSource(nameof(TestCases), new object[]{0})]
  public  void ChunkingWithCancel(IPayload payload,
                                          byte[]   bytes,
                                          int      maxChunkSize)
  {
    var res = new byte[bytes.Length];
    var idx = 0;

    var cancellationTokenSource = new CancellationTokenSource();

    Assert.ThrowsAsync<OperationCanceledException>(async () =>
                                                   {
                                                     await foreach (var rm in payload.ToChunkedByteStringAsync(maxChunkSize,
                                                                                                               cancellationTokenSource.Token))
                                                     {
                                                       rm.Memory.CopyTo(res.AsMemory(idx,
                                                                                     rm.Length));
                                                       idx += rm.Length;
                                                       cancellationTokenSource.Cancel();
                                                     }
                                                   });

  }
}
