# s14 part 1: per-process median ns per encode (UploadResultDataMessage, one 48-unit string), one-string sweep, CPUs 0,1; per configuration and path: processes, min, q1, median, q3, max, processes under 300 ns (fast) / at or above (slow)

| configuration | content | path | processes | min | q1 | median | q3 | max | fast | slow |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| default | ascii | E0 | 20 | 75 | 76 | 77 | 79 | 86 | 20 | 0 |
| default | ascii | E1R | 20 | 102 | 103 | 106 | 110 | 660 | 18 | 2 |
| default | latin1 | E0 | 20 | 95 | 98 | 104 | 113 | 119 | 20 | 0 |
| default | latin1 | E1R | 20 | 106 | 107 | 110 | 114 | 663 | 18 | 2 |
| DOTNET_TieredPGO=0 | ascii | E0 | 20 | 80 | 81 | 82 | 83 | 86 | 20 | 0 |
| DOTNET_TieredPGO=0 | ascii | E1R | 20 | 114 | 116 | 117 | 118 | 121 | 20 | 0 |
| DOTNET_TieredPGO=0 | latin1 | E0 | 20 | 99 | 101 | 105 | 111 | 113 | 20 | 0 |
| DOTNET_TieredPGO=0 | latin1 | E1R | 20 | 118 | 119 | 122 | 126 | 127 | 20 | 0 |
| DOTNET_TieredCompilation=0 | ascii | E0 | 20 | 78 | 80 | 82 | 83 | 111 | 20 | 0 |
| DOTNET_TieredCompilation=0 | ascii | E1R | 20 | 116 | 117 | 119 | 120 | 149 | 20 | 0 |
| DOTNET_TieredCompilation=0 | latin1 | E0 | 20 | 98 | 99 | 100 | 100 | 130 | 20 | 0 |
| DOTNET_TieredCompilation=0 | latin1 | E1R | 20 | 118 | 121 | 121 | 122 | 152 | 20 | 0 |
| DOTNET_TC_OnStackReplacement=0 | ascii | E0 | 20 | 75 | 76 | 77 | 78 | 93 | 20 | 0 |
| DOTNET_TC_OnStackReplacement=0 | ascii | E1R | 20 | 102 | 104 | 107 | 109 | 112 | 20 | 0 |
| DOTNET_TC_OnStackReplacement=0 | latin1 | E0 | 20 | 94 | 97 | 101 | 112 | 113 | 20 | 0 |
| DOTNET_TC_OnStackReplacement=0 | latin1 | E1R | 20 | 106 | 108 | 110 | 113 | 120 | 20 | 0 |
| DOTNET_ReadyToRun=0 | ascii | E0 | 20 | 72 | 74 | 75 | 76 | 78 | 20 | 0 |
| DOTNET_ReadyToRun=0 | ascii | E1R | 20 | 102 | 104 | 106 | 651 | 673 | 14 | 6 |
| DOTNET_ReadyToRun=0 | latin1 | E0 | 20 | 92 | 94 | 96 | 97 | 101 | 20 | 0 |
| DOTNET_ReadyToRun=0 | latin1 | E1R | 20 | 106 | 108 | 109 | 658 | 666 | 14 | 6 |
