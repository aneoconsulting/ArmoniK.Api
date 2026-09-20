```

BenchmarkDotNet v0.15.8, Linux Ubuntu 24.04.4 LTS (Noble Numbat)
Intel Xeon Processor 2.80GHz, 1 CPU, 4 logical and 4 physical cores
.NET SDK 8.0.131
  [Host]     : .NET 8.0.31 (8.0.31, 8.0.3126.42015), X64 RyuJIT x86-64-v4
  DefaultJob : .NET 8.0.31 (8.0.31, 8.0.3126.42015), X64 RyuJIT x86-64-v4


```
| Method          | Payload | Mean         | Error      | StdDev     | Ratio | RatioSD | Allocated | Alloc Ratio |
|---------------- |-------- |-------------:|-----------:|-----------:|------:|--------:|----------:|------------:|
| gp-marshaller   | P1.2    |   367.004 μs |  1.1892 μs |  1.1123 μs |  1.00 |    0.00 |         - |          NA |
| managed         | P1.2    |   154.709 μs |  0.4325 μs |  0.3612 μs |  0.42 |    0.00 |         - |          NA |
| core-ffi        | P1.2    |   191.580 μs |  0.8365 μs |  0.6985 μs |  0.52 |    0.00 |         - |          NA |
| &#39;core-ffi fill&#39; | P1.2    |    78.857 μs |  0.3425 μs |  0.3203 μs |  0.21 |    0.00 |         - |          NA |
|                 |         |              |            |            |       |         |           |             |
| gp-marshaller   | P1.3    |     7.142 μs |  0.0529 μs |  0.0469 μs |  1.00 |    0.01 |         - |          NA |
| managed         | P1.3    |     2.328 μs |  0.0062 μs |  0.0048 μs |  0.33 |    0.00 |         - |          NA |
| core-ffi        | P1.3    |     5.704 μs |  0.0055 μs |  0.0043 μs |  0.80 |    0.01 |         - |          NA |
| &#39;core-ffi fill&#39; | P1.3    |     3.660 μs |  0.0143 μs |  0.0119 μs |  0.51 |    0.00 |         - |          NA |
|                 |         |              |            |            |       |         |           |             |
| gp-marshaller   | P2.2    | 1,753.505 μs | 25.6811 μs | 24.0221 μs |  1.00 |    0.02 |   28000 B |        1.00 |
| managed         | P2.2    |   526.397 μs |  3.9735 μs |  3.5224 μs |  0.30 |    0.00 |         - |        0.00 |
| core-ffi        | P2.2    |   839.492 μs | 11.5822 μs | 10.8340 μs |  0.48 |    0.01 |         - |        0.00 |
| &#39;core-ffi fill&#39; | P2.2    |   390.899 μs |  5.6055 μs |  5.2434 μs |  0.22 |    0.00 |         - |        0.00 |
