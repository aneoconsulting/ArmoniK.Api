# s12 --sgtbench (full build), CONTAINER INSTRUMENTATION; .NET 8.0.31; affinity 0x3; rounds 6 x 40 ms blocks, order alternated

## 1. crossing microbench: raw begin + next loop, no dispatch; process CPU per decode (ns), median [min-max] over rounds; per call = per decode / calls
| probe | calls per decode | plain ns/decode | sgt ns/decode | plain ns/call | sgt ns/call |
|---|---:|---:|---:|---:|---:|
| P7.1 (DualResponse, committed vector) | 7 | 154.1 [153.6-159.5] | 142.0 [141.8-142.9] | 22.0 | 20.3 |
| empty element (ListResultsResponse 0A 00) | 2 | 31.9 [31.7-32.3] | 24.6 [24.3-24.8] | 16.0 | 12.3 |

## 2. the longest single call per row (plain import through the export's address, drop mode): per event the minimum over 30 decodes (ns), then the row's longest; events > 1 us = events whose minimum exceeds 1,000 ns
| row | bytes | calls per decode | longest call (ns) | at call # (op) | median call (ns) | calls > 1 us |
|---|---:|---:|---:|---|---:|---:|
| P1.1 | 858 | 2 | 265 | 0 (ADD) | 265 | 0 |
| P1.2 | 218121 | 8 | 11277 | 5 (ADD) | 10920 | 7 |
| P1.3 | 605 | 3 | 1130 | 0 (ADD) | 928 | 1 |
| P2.1 | 1037 | 8 | 292 | 6 (APPLY_ELEM) | 75 | 0 |
| P2.2 | 540422 | 3501 | 439 | 13 (APPLY_ELEM) | 61 | 0 |
| P2.2/latin1 | 944454 | 3501 | 455 | 13 (APPLY_ELEM) | 61 | 0 |
| P2.2/wide | 1348486 | 3501 | 465 | 13 (APPLY_ELEM) | 61 | 0 |
| P2.3 | 647024 | 876 | 425 | 258 (APPLY_ELEM) | 193 | 0 |
| P2.4 | 979465 | 561 | 797 | 120 (ADD) | 120 | 0 |
| P2.5 | 19632 | 141 | 293 | 20 (APPLY_ELEM) | 60 | 0 |
| P3.1 | 12097 | 2 | 5950 | 0 (ADD) | 5950 | 1 |
| P4.1 | 65321 | 601 | 161 | 11 (APPLY_ELEM) | 102 | 0 |
| P5.1 | 116 | 1 | 56 | 0 (APPLY) | 56 | 0 |
| P5.2 | 65620 | 1 | 56 | 0 (APPLY) | 56 | 0 |
| P5.3 | 1048660 | 1 | 57 | 0 (APPLY) | 57 | 0 |
| P5.4 | 4194390 | 1 | 56 | 0 (APPLY) | 56 | 0 |
| P6.1 | 123354 | 1401 | 171 | 29 (ADD) | 68 | 0 |
| P7.1 | 98 | 7 | 56 | 0 (ADD) | 50 | 0 |
| U-nested-before | 303 | 2 | 132 | 0 (ADD) | 132 | 0 |
| U-deep-u-repeated | 873 | 8 | 304 | 6 (APPLY_ELEM) | 60 | 0 |
| U-oneof-u-repeated | 83 | 2 | 73 | 0 (ADD) | 73 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 615 | 7 | 133 | 5 (APPLY_ELEM) | 105 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 101 | 1 | 60 | 0 (APPLY) | 60 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | 1243 | 15 | 155 | 1 (ADD) | 69 | 0 |
| U-wire-DualResponse-left-as-wt5 | 69 | 3 | 65 | 0 (ADD) | 62 | 0 |

# timer: Stopwatch (1000000000 Hz); each call's figure includes two timestamp reads (about 20-40 ns here)
