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
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

using ArmoniK.Api.Client.Internals;
using ArmoniK.Api.Client.Submitter;
using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Submitter;

using Google.Protobuf;

using Grpc.Core;

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
  [TestCaseSource(nameof(TestCases),
                  new object[]
                  {
                    3,
                  })]
  public async Task ChunkingShouldSucceed(IPayload payload,
                                          byte[]   bytes,
                                          int      maxChunkSize)
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
  [TestCaseSource(nameof(TestCases),
                  new object[]
                  {
                    0,
                  })]
  public void ChunkingWithCancel(IPayload payload,
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

  [Test]
  [TestCase(2)]
  [TestCase(3)]
  [TestCase(4)]
  [TestCase(100)]
  public async Task GetResultAsBytesAsyncShouldSucceed(int maxChunkSize)
  {
    var bytes = Encoding.ASCII.GetBytes("test_jfiejiçlkqflkljsdkljf");

    var client = new TestClient(bytes,
                                maxChunkSize);

    var res = await client.GetResultAsBytesAsync(new ResultRequest(),
                                       CancellationToken.None);

    Console.WriteLine(Encoding.ASCII.GetString(res));
    Assert.AreEqual(bytes,
                    res);
  }


  [Test]
  [TestCase(2)]
  [TestCase(3)]
  [TestCase(4)]
  [TestCase(100)]
  public async Task GetResultAsStreamAsyncShouldSucceed(int maxChunkSize)
  {
    var bytes = Encoding.ASCII.GetBytes("test_jfiejiçlkqflkljsdkljf");

    var client = new TestClient(bytes,
                                maxChunkSize);

    var stream = await client.GetResultAsStreamAsync(new ResultRequest(),
                                                  CancellationToken.None);


    var res = new byte[bytes.Length];
    var readSize = await stream.ReadAsync(res,
                           0,
                           bytes.Length);

    Console.WriteLine(Encoding.ASCII.GetString(res));

    Assert.AreNotEqual(0,
                       readSize);
    Assert.AreEqual(bytes,
                    res);
  }

  private class EnumerableAsyncStreamReader : IAsyncStreamReader<ResultReply>
  {
    private readonly IEnumerator<ResultReply> enumerator_;

    public EnumerableAsyncStreamReader(IEnumerable<ResultReply> enumerable)
    {
      enumerator_ = enumerable.GetEnumerator();
    }

    public Task<bool> MoveNext(CancellationToken cancellationToken)
      => Task.FromResult(enumerator_.MoveNext());

    public ResultReply Current
      => enumerator_.Current;
  }


  public class TestClient : gRPC.V1.Submitter.Submitter.SubmitterClient
  {
    private static Task<Metadata> GetResponse()
    {
      return Task.FromResult(new Metadata());
    }

    private readonly IAsyncStreamReader<ResultReply> streamReader_;

    public TestClient(byte[] resultData,
                      int    maxChunkSize)
    {
      var list = new List<ResultReply>();

      var start = 0;

      while (start < resultData.Length)
      {
        var chunkSize = Math.Min(maxChunkSize,
                                 resultData.Length - start);

        list.Add(new ResultReply
                 {
                   Result = new DataChunk
                            {
                              Data = UnsafeByteOperations.UnsafeWrap(resultData.AsMemory()
                                                                               .Slice(start,
                                                                                      chunkSize)),
                            },
                 });
        start += chunkSize;
      }

      list.Add(new ResultReply
               {
                 Result = new DataChunk
                          {
                            DataComplete = true,
                          },
               });

      streamReader_ = new EnumerableAsyncStreamReader(list);
    }

    public override AsyncServerStreamingCall<ResultReply> TryGetResultStream(ResultRequest request,
                                                                             CallOptions   options)
    {
      return new AsyncServerStreamingCall<ResultReply>(streamReader_,
                                                       GetResponse(),
                                                       () => Status.DefaultSuccess,
                                                       () => new Metadata(),
                                                       () =>
                                                       {
                                                       });
    }
  }
}
