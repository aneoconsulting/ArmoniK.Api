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
using System.Text;
using System.Text.Json;

namespace ArmoniK.Api.TransportOptionsGenerator
{
  /// <summary>
  ///   Command line around <see cref="Generator" />.
  /// </summary>
  public static class Program
  {
    private const string Usage = @"Generates the C# options class of the native transport.

Usage:
  dotnet run --project packages/csharp/tools/ArmoniK.Api.TransportOptionsGenerator -- \
    --schema <path to the JSON schema> --output <path to the .cs file to write>

Options:
  --schema <path>  JSON schema of the armonik-transport option vocabulary, as printed by
                   `cargo run -p armonik-transport --features schema --example generate_schema`.
  --output <path>  File to write; its directory has to exist.
  -h, --help       Prints this text.

The same schema always gives the same bytes, so regenerating and diffing is a check.";

    /// <summary>
    ///   Reads the schema named on the command line and writes the options class.
    /// </summary>
    /// <param name="args">The command line arguments.</param>
    /// <returns>
    ///   0 on success, 1 on a bad command line, 2 on a schema this generator cannot read, 3 on an
    ///   output it cannot write.
    /// </returns>
    public static int Main(string[] args)
    {
      string? schemaPath = null;
      string? outputPath = null;

      for (var i = 0; i < args.Length; i++)
      {
        switch (args[i])
        {
          case "-h":
          case "--help":
            Console.Out.WriteLine(Usage);
            return 0;
          case "--schema" when i + 1 < args.Length:
            schemaPath = args[++i];
            break;
          case "--output" when i + 1 < args.Length:
            outputPath = args[++i];
            break;
          default:
            Console.Error.WriteLine($"Unexpected argument '{args[i]}'.");
            Console.Error.WriteLine(Usage);
            return 1;
        }
      }

      if (schemaPath is null || outputPath is null)
      {
        Console.Error.WriteLine("Both --schema and --output are required.");
        Console.Error.WriteLine(Usage);
        return 1;
      }

      string generated;

      try
      {
        generated = Generator.Generate(File.ReadAllText(schemaPath));
      }
      catch (Exception e) when (IsFileFailure(e) || e is InvalidOperationException or JsonException)
      {
        Console.Error.WriteLine($"Cannot generate from '{schemaPath}': {e.Message}");
        return 2;
      }

      try
      {
        // No byte order mark, and the newlines the generator chose: the output is compared byte for
        // byte with the file already committed.
        File.WriteAllText(outputPath,
                          generated,
                          new UTF8Encoding(false));
      }
      catch (Exception e) when (IsFileFailure(e))
      {
        Console.Error.WriteLine($"Cannot write '{outputPath}': {e.Message}");
        return 3;
      }

      return 0;
    }

    // A path can fail on its shape, on its permissions or on the drive, and a caller wants the
    // reason on one line rather than a stack trace.
    private static bool IsFileFailure(Exception e)
      => e is IOException or UnauthorizedAccessException or ArgumentException or NotSupportedException;
  }
}
