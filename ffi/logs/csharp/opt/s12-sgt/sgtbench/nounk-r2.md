# s12 --sgtbench (no-unknown build), CONTAINER INSTRUMENTATION; .NET 8.0.31; affinity 0x3; rounds 6 x 40 ms blocks, order alternated

## 1. crossing microbench: raw begin + next loop, no dispatch; process CPU per decode (ns), median [min-max] over rounds; per call = per decode / calls
| probe | calls per decode | plain ns/decode | sgt ns/decode | plain ns/call | sgt ns/call |
|---|---:|---:|---:|---:|---:|
| P7.1 (DualResponse, committed vector) | 7 | 146.5 [144.3-146.9] | 138.5 [133.6-140.4] | 20.9 | 19.8 |
| empty element (ListResultsResponse 0A 00) | 2 | 32.5 [32.1-33.2] | 23.3 [22.5-27.7] | 16.3 | 11.6 |

## 2. the longest single call per row (plain import through the export's address, no-unknown mode): per event the minimum over 30 decodes (ns), then the row's longest; events > 1 us = events whose minimum exceeds 1,000 ns
| row | bytes | calls per decode | longest call (ns) | at call # (op) | median call (ns) | calls > 1 us |
|---|---:|---:|---:|---|---:|---:|
| P1.1 | 858 | 2 | 265 | 0 (ADD) | 265 | 0 |
| P1.2 | 218121 | 5 | 17745 | 1 (ADD) | 17220 | 4 |
| P1.3 | 605 | 3 | 1183 | 0 (ADD) | 253 | 1 |
| P2.1 | 1037 | 8 | 300 | 6 (APPLY_ELEM) | 63 | 0 |
| P2.2 | 540422 | 3501 | 444 | 13 (APPLY_ELEM) | 56 | 0 |
| P2.2/latin1 | 944454 | 3501 | 450 | 13 (APPLY_ELEM) | 56 | 0 |
| P2.2/wide | 1348486 | 3501 | 462 | 13 (APPLY_ELEM) | 57 | 0 |
| P2.3 | 647024 | 876 | 432 | 258 (APPLY_ELEM) | 194 | 0 |
| P2.4 | 979465 | 561 | 842 | 204 (ADD) | 115 | 0 |
| P2.5 | 19632 | 141 | 298 | 132 (APPLY_ELEM) | 56 | 0 |
| P3.1 | 12097 | 2 | 5962 | 0 (ADD) | 5962 | 1 |
| P4.1 | 65321 | 601 | 159 | 11 (APPLY_ELEM) | 102 | 0 |
| P5.1 | 116 | 1 | 52 | 0 (APPLY) | 52 | 0 |
| P5.2 | 65620 | 1 | 53 | 0 (APPLY) | 53 | 0 |
| P5.3 | 1048660 | 1 | 55 | 0 (APPLY) | 55 | 0 |
| P5.4 | 4194390 | 1 | 56 | 0 (APPLY) | 56 | 0 |
| P6.1 | 123354 | 1401 | 174 | 260 (ADD) | 61 | 0 |
| P7.1 | 98 | 7 | 51 | 0 (ADD) | 48 | 0 |
| U-nested-before | 303 | 2 | 136 | 0 (ADD) | 136 | 0 |
| U-deep-u-repeated | 873 | 8 | 320 | 6 (APPLY_ELEM) | 50 | 0 |
| U-oneof-u-repeated | 83 | 2 | 76 | 0 (ADD) | 76 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 615 | 7 | 127 | 5 (APPLY_ELEM) | 103 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 101 | 1 | 56 | 0 (APPLY) | 56 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | 1243 | 15 | 166 | 1 (ADD) | 62 | 0 |
| U-wire-DualResponse-left-as-wt5 | 69 | 3 | 63 | 0 (ADD) | 60 | 0 |

# timer: Stopwatch (1000000000 Hz); each call's figure includes two timestamp reads (about 20-40 ns here)
