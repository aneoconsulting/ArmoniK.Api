```

BenchmarkDotNet v0.15.8, Linux Ubuntu 24.04.4 LTS (Noble Numbat)
Intel Xeon Processor 2.80GHz, 1 CPU, 4 logical and 4 physical cores
.NET SDK 8.0.131
  [Host]     : .NET 8.0.31 (8.0.31, 8.0.3126.42015), X64 RyuJIT x86-64-v4
  DefaultJob : .NET 8.0.31 (8.0.31, 8.0.3126.42015), X64 RyuJIT x86-64-v4


```
| Method        | Payload | Mean     | Error     | StdDev    | Ratio | RatioSD | Gen0     | Gen1     | Allocated | Alloc Ratio |
|-------------- |-------- |---------:|----------:|----------:|------:|--------:|---------:|---------:|----------:|------------:|
| gp-parse-seq  | P2.2    | 2.112 ms | 0.0398 ms | 0.0372 ms |  1.00 |    0.02 | 132.8125 | 128.9063 |   2.21 MB |        1.00 |
| managed-parse | P2.2    | 1.772 ms | 0.0351 ms | 0.0913 ms |  0.84 |    0.05 | 121.0938 | 119.1406 |   2.02 MB |        0.91 |
| core-ffi      | P2.2    | 1.594 ms | 0.0318 ms | 0.0367 ms |  0.75 |    0.02 | 121.0938 |  85.9375 |   2.02 MB |        0.91 |
