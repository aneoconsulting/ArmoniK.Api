# s12 --sgtbench (no-unknown build), CONTAINER INSTRUMENTATION; .NET 8.0.31; affinity 0x3; rounds 6 x 40 ms blocks, order alternated

## 1. crossing microbench: raw begin + next loop, no dispatch; process CPU per decode (ns), median [min-max] over rounds; per call = per decode / calls
| probe | calls per decode | plain ns/decode | sgt ns/decode | plain ns/call | sgt ns/call |
|---|---:|---:|---:|---:|---:|
| P7.1 (DualResponse, committed vector) | 7 | 145.2 [144.1-152.6] | 136.6 [134.9-139.7] | 20.7 | 19.5 |
| empty element (ListResultsResponse 0A 00) | 2 | 31.3 [31.1-31.7] | 22.2 [22.0-22.7] | 15.7 | 11.1 |

## 2. the longest single call per row (plain import through the export's address, no-unknown mode): per event the minimum over 30 decodes (ns), then the row's longest; events > 1 us = events whose minimum exceeds 1,000 ns
| row | bytes | calls per decode | longest call (ns) | at call # (op) | median call (ns) | calls > 1 us |
|---|---:|---:|---:|---|---:|---:|
| P1.1 | 858 | 2 | 261 | 0 (ADD) | 261 | 0 |
| P1.2 | 218121 | 5 | 17532 | 1 (ADD) | 17088 | 4 |
| P1.3 | 605 | 3 | 1254 | 0 (ADD) | 256 | 1 |
| P2.1 | 1037 | 8 | 296 | 6 (APPLY_ELEM) | 56 | 0 |
| P2.2 | 540422 | 3501 | 466 | 13 (APPLY_ELEM) | 57 | 0 |
| P2.2/latin1 | 944454 | 3501 | 479 | 13 (APPLY_ELEM) | 57 | 0 |
| P2.2/wide | 1348486 | 3501 | 471 | 13 (APPLY_ELEM) | 57 | 0 |
| P2.3 | 647024 | 876 | 428 | 258 (APPLY_ELEM) | 194 | 0 |
| P2.4 | 979465 | 561 | 842 | 260 (ADD) | 116 | 0 |
| P2.5 | 19632 | 141 | 305 | 20 (APPLY_ELEM) | 56 | 0 |
| P3.1 | 12097 | 2 | 6082 | 0 (ADD) | 6082 | 1 |
| P4.1 | 65321 | 601 | 158 | 11 (APPLY_ELEM) | 105 | 0 |
| P5.1 | 116 | 1 | 53 | 0 (APPLY) | 53 | 0 |
| P5.2 | 65620 | 1 | 54 | 0 (APPLY) | 54 | 0 |
| P5.3 | 1048660 | 1 | 55 | 0 (APPLY) | 55 | 0 |
| P5.4 | 4194390 | 1 | 57 | 0 (APPLY) | 57 | 0 |
| P6.1 | 123354 | 1401 | 174 | 22 (ADD) | 62 | 0 |
| P7.1 | 98 | 7 | 55 | 0 (ADD) | 48 | 0 |
| U-nested-before | 303 | 2 | 136 | 0 (ADD) | 136 | 0 |
| U-deep-u-repeated | 873 | 8 | 315 | 6 (APPLY_ELEM) | 52 | 0 |
| U-oneof-u-repeated | 83 | 2 | 75 | 0 (ADD) | 75 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 615 | 7 | 137 | 5 (APPLY_ELEM) | 107 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 101 | 1 | 56 | 0 (APPLY) | 56 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | 1243 | 15 | 157 | 1 (ADD) | 62 | 0 |
| U-wire-DualResponse-left-as-wt5 | 69 | 3 | 67 | 0 (ADD) | 61 | 0 |

# timer: Stopwatch (1000000000 Hz); each call's figure includes two timestamp reads (about 20-40 ns here)
