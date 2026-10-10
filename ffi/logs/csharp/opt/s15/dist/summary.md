# s15: per-process median ns per encode (UploadResultDataMessage, one 48-unit string), one-string sweep, CPUs 0,1, interleaved; per configuration and path: processes, min, q1, median, q3, max, processes under 300 ns (fast) / at or above (slow). CONTAINER INSTRUMENTATION.

| configuration | content | path | processes | min | q1 | median | q3 | max | fast | slow |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| D26 code, default JIT configuration | ascii | E0 | 60 | 76 | 78 | 79 | 80 | 417 | 59 | 1 |
| D26 code, default JIT configuration | ascii | E1R | 60 | 94 | 97 | 98 | 101 | 387 | 59 | 1 |
| D26 code, default JIT configuration | latin1 | E0 | 60 | 95 | 100 | 108 | 112 | 136 | 60 | 0 |
| D26 code, default JIT configuration | latin1 | E1R | 60 | 97 | 100 | 102 | 105 | 133 | 60 | 0 |
| D26 code, DOTNET_TieredPGO=0 | ascii | E0 | 60 | 79 | 82 | 83 | 85 | 102 | 60 | 0 |
| D26 code, DOTNET_TieredPGO=0 | ascii | E1R | 60 | 102 | 106 | 107 | 109 | 124 | 60 | 0 |
| D26 code, DOTNET_TieredPGO=0 | latin1 | E0 | 60 | 100 | 101 | 102 | 104 | 129 | 60 | 0 |
| D26 code, DOTNET_TieredPGO=0 | latin1 | E1R | 60 | 106 | 110 | 111 | 114 | 140 | 60 | 0 |
| code before D26 (control), default JIT configuration | ascii | E0 | 60 | 74 | 77 | 78 | 79 | 108 | 60 | 0 |
| code before D26 (control), default JIT configuration | ascii | E1R | 60 | 98 | 103 | 104 | 107 | 696 | 52 | 8 |
| code before D26 (control), default JIT configuration | latin1 | E0 | 60 | 93 | 96 | 99 | 111 | 124 | 60 | 0 |
| code before D26 (control), default JIT configuration | latin1 | E1R | 60 | 103 | 106 | 108 | 112 | 704 | 52 | 8 |
