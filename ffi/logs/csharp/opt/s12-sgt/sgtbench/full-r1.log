# s12 --sgtbench (full build), CONTAINER INSTRUMENTATION; .NET 8.0.31; affinity 0x3; rounds 6 x 40 ms blocks, order alternated

## 1. crossing microbench: raw begin + next loop, no dispatch; process CPU per decode (ns), median [min-max] over rounds; per call = per decode / calls
| probe | calls per decode | plain ns/decode | sgt ns/decode | plain ns/call | sgt ns/call |
|---|---:|---:|---:|---:|---:|
| P7.1 (DualResponse, committed vector) | 7 | 155.7 [154.9-157.1] | 143.8 [142.4-144.5] | 22.2 | 20.5 |
| empty element (ListResultsResponse 0A 00) | 2 | 32.1 [31.9-32.5] | 24.7 [24.5-25.4] | 16.1 | 12.3 |

## 2. the longest single call per row (plain import through the export's address, drop mode): per event the minimum over 30 decodes (ns), then the row's longest; events > 1 us = events whose minimum exceeds 1,000 ns
| row | bytes | calls per decode | longest call (ns) | at call # (op) | median call (ns) | calls > 1 us |
|---|---:|---:|---:|---|---:|---:|
| P1.1 | 858 | 2 | 270 | 0 (ADD) | 270 | 0 |
| P1.2 | 218121 | 8 | 11253 | 2 (ADD) | 10869 | 7 |
| P1.3 | 605 | 3 | 1091 | 0 (ADD) | 884 | 1 |
| P2.1 | 1037 | 8 | 282 | 6 (APPLY_ELEM) | 60 | 0 |
| P2.2 | 540422 | 3501 | 430 | 13 (APPLY_ELEM) | 60 | 0 |
| P2.2/latin1 | 944454 | 3501 | 433 | 27 (APPLY_ELEM) | 60 | 0 |
| P2.2/wide | 1348486 | 3501 | 441 | 13 (APPLY_ELEM) | 60 | 0 |
| P2.3 | 647024 | 876 | 420 | 20 (APPLY_ELEM) | 193 | 0 |
| P2.4 | 979465 | 561 | 787 | 514 (ADD) | 123 | 0 |
| P2.5 | 19632 | 141 | 294 | 20 (APPLY_ELEM) | 60 | 0 |
| P3.1 | 12097 | 2 | 6059 | 0 (ADD) | 6059 | 1 |
| P4.1 | 65321 | 601 | 162 | 11 (APPLY_ELEM) | 102 | 0 |
| P5.1 | 116 | 1 | 56 | 0 (APPLY) | 56 | 0 |
| P5.2 | 65620 | 1 | 58 | 0 (APPLY) | 58 | 0 |
| P5.3 | 1048660 | 1 | 57 | 0 (APPLY) | 57 | 0 |
| P5.4 | 4194390 | 1 | 57 | 0 (APPLY) | 57 | 0 |
| P6.1 | 123354 | 1401 | 170 | 8 (ADD) | 67 | 0 |
| P7.1 | 98 | 7 | 55 | 0 (ADD) | 50 | 0 |
| U-nested-before | 303 | 2 | 131 | 0 (ADD) | 131 | 0 |
| U-deep-u-repeated | 873 | 8 | 303 | 6 (APPLY_ELEM) | 60 | 0 |
| U-oneof-u-repeated | 83 | 2 | 76 | 0 (ADD) | 76 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 615 | 7 | 136 | 5 (APPLY_ELEM) | 104 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 101 | 1 | 59 | 0 (APPLY) | 59 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | 1243 | 15 | 150 | 1 (ADD) | 69 | 0 |
| U-wire-DualResponse-left-as-wt5 | 69 | 3 | 66 | 0 (ADD) | 62 | 0 |

# timer: Stopwatch (1000000000 Hz); each call's figure includes two timestamp reads (about 20-40 ns here)
