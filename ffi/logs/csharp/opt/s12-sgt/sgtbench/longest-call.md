# s12: the longest single FSM call per row (plain import; per event the minimum over 30 decodes, then the row's longest), two processes per build; ns

| row | bytes | calls per decode | drop: longest (r1 / r2) | drop: at call (op) | drop: calls > 1 us | no-unknown: longest (r1 / r2) | no-unknown: calls > 1 us |
|---|---:|---:|---:|---|---:|---:|---:|
| P1.1 | 858 | 2 | 270 / 265 | 0 (ADD) | 0 / 0 | 261 / 265 | 0 / 0 |
| P1.2 | 218121 | 8 | 11253 / 11277 | 2 (ADD) | 7 / 7 | 17532 / 17745 | 4 / 4 |
| P1.3 | 605 | 3 | 1091 / 1130 | 0 (ADD) | 1 / 1 | 1254 / 1183 | 1 / 1 |
| P2.1 | 1037 | 8 | 282 / 292 | 6 (APPLY_ELEM) | 0 / 0 | 296 / 300 | 0 / 0 |
| P2.2 | 540422 | 3501 | 430 / 439 | 13 (APPLY_ELEM) | 0 / 0 | 466 / 444 | 0 / 0 |
| P2.2/latin1 | 944454 | 3501 | 433 / 455 | 27 (APPLY_ELEM) | 0 / 0 | 479 / 450 | 0 / 0 |
| P2.2/wide | 1348486 | 3501 | 441 / 465 | 13 (APPLY_ELEM) | 0 / 0 | 471 / 462 | 0 / 0 |
| P2.3 | 647024 | 876 | 420 / 425 | 20 (APPLY_ELEM) | 0 / 0 | 428 / 432 | 0 / 0 |
| P2.4 | 979465 | 561 | 787 / 797 | 514 (ADD) | 0 / 0 | 842 / 842 | 0 / 0 |
| P2.5 | 19632 | 141 | 294 / 293 | 20 (APPLY_ELEM) | 0 / 0 | 305 / 298 | 0 / 0 |
| P3.1 | 12097 | 2 | 6059 / 5950 | 0 (ADD) | 1 / 1 | 6082 / 5962 | 1 / 1 |
| P4.1 | 65321 | 601 | 162 / 161 | 11 (APPLY_ELEM) | 0 / 0 | 158 / 159 | 0 / 0 |
| P5.1 | 116 | 1 | 56 / 56 | 0 (APPLY) | 0 / 0 | 53 / 52 | 0 / 0 |
| P5.2 | 65620 | 1 | 58 / 56 | 0 (APPLY) | 0 / 0 | 54 / 53 | 0 / 0 |
| P5.3 | 1048660 | 1 | 57 / 57 | 0 (APPLY) | 0 / 0 | 55 / 55 | 0 / 0 |
| P5.4 | 4194390 | 1 | 57 / 56 | 0 (APPLY) | 0 / 0 | 57 / 56 | 0 / 0 |
| P6.1 | 123354 | 1401 | 170 / 171 | 8 (ADD) | 0 / 0 | 174 / 174 | 0 / 0 |
| P7.1 | 98 | 7 | 55 / 56 | 0 (ADD) | 0 / 0 | 55 / 51 | 0 / 0 |
| U-nested-before | 303 | 2 | 131 / 132 | 0 (ADD) | 0 / 0 | 136 / 136 | 0 / 0 |
| U-deep-u-repeated | 873 | 8 | 303 / 304 | 6 (APPLY_ELEM) | 0 / 0 | 315 / 320 | 0 / 0 |
| U-oneof-u-repeated | 83 | 2 | 76 / 73 | 0 (ADD) | 0 / 0 | 75 / 76 | 0 / 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 615 | 7 | 136 / 133 | 5 (APPLY_ELEM) | 0 / 0 | 137 / 127 | 0 / 0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 101 | 1 | 59 / 60 | 0 (APPLY) | 0 / 0 | 56 / 56 | 0 / 0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | 1243 | 15 | 150 / 155 | 1 (ADD) | 0 / 0 | 157 / 166 | 0 / 0 |
| U-wire-DualResponse-left-as-wt5 | 69 | 3 | 66 / 65 | 0 (ADD) | 0 / 0 | 67 / 63 | 0 / 0 |
