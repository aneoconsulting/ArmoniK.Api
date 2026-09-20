```

BenchmarkDotNet v0.15.8, Linux Ubuntu 24.04.4 LTS (Noble Numbat)
Intel Xeon Processor 2.80GHz, 1 CPU, 4 logical and 4 physical cores
.NET SDK 8.0.131
  [Host]     : .NET 8.0.31 (8.0.31, 8.0.3126.42015), X64 RyuJIT x86-64-v4
  DefaultJob : .NET 8.0.31 (8.0.31, 8.0.3126.42015), X64 RyuJIT x86-64-v4


```
| Method        | Payload | Mean       | Error    | StdDev   | Ratio | Allocated | Alloc Ratio |
|-------------- |-------- |-----------:|---------:|---------:|------:|----------:|------------:|
| gp-marshaller | P2.2    | 1,750.0 μs | 19.72 μs | 18.44 μs |  1.00 |   28000 B |        1.00 |
| managed       | P2.2    |   527.8 μs |  3.17 μs |  2.64 μs |  0.30 |         - |        0.00 |
| core-ffi      | P2.2    |   837.0 μs |  5.92 μs |  4.94 μs |  0.48 |         - |        0.00 |
