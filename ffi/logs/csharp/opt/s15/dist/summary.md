# s15: per-process median ns per encode (UploadResultDataMessage, one 48-unit string), one-string sweep, CPUs 0,1, interleaved; per configuration and path: processes, min, q1, median, q3, max, processes under 300 ns (fast) / at or above (slow). CONTAINER INSTRUMENTATION.

| configuration | content | path | processes | min | q1 | median | q3 | max | fast | slow |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| D26 code, default JIT configuration | ascii | E0 | 20 | 77 | 78 | 79 | 82 | 114 | 20 | 0 |
| D26 code, default JIT configuration | ascii | E1R | 20 | 94 | 96 | 97 | 101 | 129 | 20 | 0 |
| D26 code, default JIT configuration | latin1 | E0 | 20 | 96 | 100 | 104 | 112 | 136 | 20 | 0 |
| D26 code, default JIT configuration | latin1 | E1R | 20 | 98 | 100 | 100 | 103 | 133 | 20 | 0 |
| D26 code, DOTNET_TieredPGO=0 | ascii | E0 | 20 | 80 | 82 | 83 | 86 | 94 | 20 | 0 |
| D26 code, DOTNET_TieredPGO=0 | ascii | E1R | 20 | 104 | 106 | 107 | 108 | 118 | 20 | 0 |
| D26 code, DOTNET_TieredPGO=0 | latin1 | E0 | 20 | 100 | 100 | 102 | 103 | 112 | 20 | 0 |
| D26 code, DOTNET_TieredPGO=0 | latin1 | E1R | 20 | 109 | 110 | 111 | 116 | 120 | 20 | 0 |
| code before D26 (control), default JIT configuration | ascii | E0 | 20 | 76 | 77 | 79 | 82 | 108 | 20 | 0 |
| code before D26 (control), default JIT configuration | ascii | E1R | 20 | 98 | 102 | 104 | 106 | 696 | 19 | 1 |
| code before D26 (control), default JIT configuration | latin1 | E0 | 20 | 93 | 97 | 109 | 111 | 124 | 20 | 0 |
| code before D26 (control), default JIT configuration | latin1 | E1R | 20 | 103 | 105 | 107 | 110 | 704 | 19 | 1 |
