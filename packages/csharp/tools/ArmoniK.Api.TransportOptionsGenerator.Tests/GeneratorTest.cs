// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2026. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System;
using System.IO;

using NUnit.Framework;

namespace ArmoniK.Api.TransportOptionsGenerator.Tests
{
  /// <summary>
  ///   Pins the generator against a snapshot of the schema and the file it writes from it.
  /// </summary>
  [TestFixture]
  public class GeneratorTest
  {
    private static string FixturePath(string name)
      => Path.Combine(TestContext.CurrentContext.TestDirectory,
                      "Fixtures",
                      name);

    private static string Schema
      => File.ReadAllText(FixturePath("http_config.schema.json"));

    [Test]
    public void GeneratedFileMatchesTheGoldenFile()
      => Assert.That(Generator.Generate(Schema),
                     Is.EqualTo(File.ReadAllText(FixturePath("TransportOptions.cs"))),
                     "Regenerate the golden file with dotnet run --project packages/csharp/tools/ArmoniK.Api.TransportOptionsGenerator");

    [Test]
    public void GenerationIsReproducible()
      => Assert.That(Generator.Generate(Schema),
                     Is.EqualTo(Generator.Generate(Schema)));

    [Test]
    public void AnOptionTypedOtherThanTextIsRefused()
      => Assert.That(() => Generator.Generate(@"{""properties"":{""MaxAttempts"":{""type"":""integer""}}}"),
                     Throws.InstanceOf<InvalidOperationException>());

    /// <summary>
    ///   A name that is not PascalCase is not the vocabulary's, and may be a C# keyword.
    /// </summary>
    [Test]
    public void AnOptionNotSpelledInPascalCaseIsRefused()
      => Assert.That(() => Generator.Generate(@"{""properties"":{""class"":{""type"":""string""}}}"),
                     Throws.InstanceOf<InvalidOperationException>());

    /// <summary>
    ///   The schema describes <c>ProxyAddress</c> twice, once per proxy shape. The generated
    ///   documentation is the first of the two, which describes the option rather than a shape.
    /// </summary>
    [Test]
    public void ARepeatedOptionKeepsItsFirstDescription()
    {
      var generated = Generator.Generate(Schema);

      Assert.That(generated,
                  Does.Contain("Where to find the proxy"));
      Assert.That(generated,
                  Does.Not.Contain("The proxy URL, credentials included."));
    }
  }
}
