# reset-on-entry: explicit reset vs reset on entry (container instrumentation, absolute times)

3 launches (`../../logs/rust/opt/reset-on-entry/bench`); each cell: the median over launches of the per-launch median, (the range of the per-launch medians). One process per launch, both paths interleaved in it on the reset-on-entry core (the explicit path with the core's switch off on its contexts).

## FSM decode-read

| input | mode | explicit reset | reset on entry |
|---|---|---|---|
| P1.1 | drop | 1.32 us (1.30 us - 1.37 us) | 1.31 us (1.31 us - 1.36 us) |
| P1.1 | retain | 1.38 us (1.33 us - 1.44 us) | 1.38 us (1.33 us - 1.43 us) |
| P1.2 | drop | 439.53 us (435.56 us - 456.56 us) | 438.83 us (431.99 us - 443.92 us) |
| P1.2 | retain | 447.81 us (447.73 us - 448.15 us) | 437.03 us (434.14 us - 442.14 us) |
| P1.3 | drop | 19.80 us (19.64 us - 20.36 us) | 20.04 us (19.80 us - 20.35 us) |
| P1.3 | retain | 19.80 us (19.77 us - 19.86 us) | 19.78 us (19.64 us - 19.88 us) |
| P2.1 | drop | 2.65 us (2.62 us - 2.72 us) | 2.66 us (2.58 us - 2.77 us) |
| P2.1 | retain | 2.73 us (2.67 us - 2.73 us) | 2.71 us (2.65 us - 2.73 us) |
| P2.2 | drop | 1.970 ms (1.959 ms - 1.986 ms) | 1.951 ms (1.901 ms - 1.963 ms) |
| P2.2 | retain | 1.944 ms (1.931 ms - 2.002 ms) | 1.932 ms (1.890 ms - 1.964 ms) |
| P2.2/latin1 | drop | 1.963 ms (1.956 ms - 2.014 ms) | 1.904 ms (1.901 ms - 1.996 ms) |
| P2.2/latin1 | retain | 2.024 ms (1.994 ms - 2.063 ms) | 1.970 ms (1.958 ms - 1.986 ms) |
| P2.2/wide | drop | 2.234 ms (2.230 ms - 2.267 ms) | 2.144 ms (2.133 ms - 2.146 ms) |
| P2.2/wide | retain | 2.184 ms (2.177 ms - 2.268 ms) | 2.100 ms (2.092 ms - 2.154 ms) |
| P2.3 | drop | 1.302 ms (1.299 ms - 1.347 ms) | 1.281 ms (1.259 ms - 1.299 ms) |
| P2.3 | retain | 1.323 ms (1.302 ms - 1.338 ms) | 1.282 ms (1.279 ms - 1.309 ms) |
| P2.4 | drop | 1.758 ms (1.720 ms - 1.839 ms) | 1.693 ms (1.690 ms - 1.785 ms) |
| P2.4 | retain | 1.741 ms (1.739 ms - 1.762 ms) | 1.680 ms (1.670 ms - 1.713 ms) |
| P2.5 | drop | 63.88 us (63.88 us - 64.93 us) | 63.53 us (63.35 us - 63.68 us) |
| P2.5 | retain | 63.39 us (63.18 us - 68.48 us) | 62.86 us (62.79 us - 67.97 us) |
| P3.1 | drop | 32.02 us (31.54 us - 32.33 us) | 31.92 us (31.34 us - 32.30 us) |
| P3.1 | retain | 31.33 us (31.07 us - 32.52 us) | 31.46 us (31.32 us - 31.80 us) |
| P4.1 | drop | 340.08 us (325.69 us - 340.83 us) | 338.72 us (329.25 us - 342.48 us) |
| P4.1 | retain | 335.95 us (326.14 us - 343.66 us) | 336.69 us (328.25 us - 345.85 us) |
| P5.1 | drop | 131.3 ns (131.0 ns - 132.0 ns) | 132.1 ns (132.1 ns - 132.6 ns) |
| P5.1 | retain | 141.0 ns (140.2 ns - 141.7 ns) | 141.5 ns (140.9 ns - 141.9 ns) |
| P5.2 | drop | 1.94 us (1.92 us - 1.95 us) | 1.94 us (1.93 us - 1.96 us) |
| P5.2 | retain | 1.99 us (1.98 us - 2.02 us) | 1.98 us (1.97 us - 2.01 us) |
| P5.3 | drop | 45.58 us (44.59 us - 47.79 us) | 45.40 us (44.01 us - 46.57 us) |
| P5.3 | retain | 46.50 us (43.95 us - 47.01 us) | 44.89 us (43.97 us - 47.02 us) |
| P5.4 | drop | 348.11 us (339.77 us - 349.09 us) | 336.69 us (334.04 us - 338.17 us) |
| P5.4 | retain | 342.51 us (342.10 us - 343.42 us) | 333.38 us (332.11 us - 334.52 us) |
| P6.1 | drop | 198.74 us (197.09 us - 201.74 us) | 197.06 us (196.96 us - 198.51 us) |
| P6.1 | retain | 208.80 us (208.77 us - 215.01 us) | 195.94 us (195.66 us - 197.80 us) |
| P7.1 | drop | 433.4 ns (430.8 ns - 434.7 ns) | 428.3 ns (424.8 ns - 429.9 ns) |
| P7.1 | retain | 442.7 ns (441.4 ns - 446.8 ns) | 444.3 ns (440.1 ns - 450.0 ns) |
| U-deep-u-repeated | drop | 2.43 us (2.39 us - 2.50 us) | 2.45 us (2.39 us - 2.51 us) |
| U-deep-u-repeated | retain | 2.63 us (2.54 us - 2.63 us) | 2.62 us (2.55 us - 2.63 us) |
| U-nested-before | drop | 384.1 ns (381.2 ns - 395.3 ns) | 380.3 ns (378.7 ns - 389.0 ns) |
| U-nested-before | retain | 551.3 ns (549.4 ns - 559.5 ns) | 537.8 ns (534.7 ns - 547.5 ns) |
| U-oneof-u-repeated | drop | 159.7 ns (159.3 ns - 162.9 ns) | 159.9 ns (158.3 ns - 160.8 ns) |
| U-oneof-u-repeated | retain | 210.3 ns (208.6 ns - 211.7 ns) | 210.5 ns (210.5 ns - 212.8 ns) |
| U-wire-DualResponse-left-as-wt5 | drop | 286.9 ns (284.4 ns - 289.7 ns) | 285.0 ns (284.5 ns - 285.8 ns) |
| U-wire-DualResponse-left-as-wt5 | retain | 333.8 ns (330.0 ns - 335.1 ns) | 324.1 ns (321.3 ns - 324.8 ns) |
| U-wire-ListMetricsResponse-batches-as-wt0 | drop | 1.60 us (1.59 us - 1.62 us) | 1.57 us (1.56 us - 1.59 us) |
| U-wire-ListMetricsResponse-batches-as-wt0 | retain | 1.72 us (1.71 us - 1.76 us) | 1.67 us (1.64 us - 1.69 us) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | drop | 2.34 us (2.27 us - 2.41 us) | 2.33 us (2.33 us - 2.39 us) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | retain | 2.44 us (2.41 us - 2.44 us) | 2.43 us (2.40 us - 2.47 us) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | drop | 139.1 ns (138.1 ns - 139.7 ns) | 140.2 ns (138.7 ns - 140.9 ns) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | retain | 180.6 ns (179.5 ns - 182.5 ns) | 182.5 ns (182.2 ns - 184.2 ns) |

## encode, end state reused-buffer (= transport-ready-core's op)

| input | mode | explicit reset | reset on entry |
|---|---|---|---|
| P1.1 | drop | 275.4 ns (274.7 ns - 285.2 ns) | 281.1 ns (279.3 ns - 290.2 ns) |
| P1.1 | retain | 295.7 ns (287.0 ns - 309.0 ns) | 292.5 ns (281.0 ns - 294.5 ns) |
| P1.2 | drop | 86.18 us (85.38 us - 87.13 us) | 86.07 us (85.07 us - 87.92 us) |
| P1.2 | retain | 88.74 us (88.57 us - 88.86 us) | 89.17 us (88.92 us - 89.22 us) |
| P1.3 | drop | 3.82 us (3.81 us - 3.92 us) | 3.77 us (3.77 us - 3.80 us) |
| P1.3 | retain | 3.98 us (3.97 us - 3.98 us) | 3.96 us (3.91 us - 3.97 us) |
| P2.1 | drop | 465.7 ns (462.6 ns - 475.3 ns) | 465.4 ns (462.4 ns - 476.9 ns) |
| P2.1 | retain | 485.4 ns (480.1 ns - 496.9 ns) | 488.3 ns (486.6 ns - 497.2 ns) |
| P2.2 | drop | 295.24 us (294.95 us - 297.51 us) | 298.21 us (296.45 us - 298.43 us) |
| P2.2 | retain | 318.62 us (318.49 us - 324.18 us) | 316.76 us (315.69 us - 318.89 us) |
| P2.2/latin1 | drop | 303.33 us (299.14 us - 312.34 us) | 303.63 us (296.72 us - 304.83 us) |
| P2.2/latin1 | retain | 315.17 us (314.94 us - 320.64 us) | 316.65 us (315.56 us - 319.47 us) |
| P2.2/wide | drop | 307.37 us (306.36 us - 311.89 us) | 312.20 us (309.79 us - 316.72 us) |
| P2.2/wide | retain | 311.26 us (309.04 us - 321.65 us) | 312.13 us (310.93 us - 322.82 us) |
| P2.3 | drop | 158.09 us (157.24 us - 161.70 us) | 157.98 us (155.78 us - 161.26 us) |
| P2.3 | retain | 162.69 us (160.01 us - 164.83 us) | 161.63 us (158.11 us - 164.51 us) |
| P2.4 | drop | 250.72 us (245.26 us - 252.84 us) | 241.62 us (240.71 us - 256.15 us) |
| P2.4 | retain | 246.72 us (246.40 us - 251.58 us) | 247.61 us (246.76 us - 249.40 us) |
| P2.5 | drop | 8.66 us (8.59 us - 8.72 us) | 8.56 us (8.55 us - 8.56 us) |
| P2.5 | retain | 8.78 us (8.70 us - 8.92 us) | 8.70 us (8.62 us - 9.09 us) |
| P3.1 | drop | 6.29 us (6.20 us - 6.34 us) | 6.20 us (6.15 us - 6.34 us) |
| P3.1 | retain | 6.57 us (6.50 us - 6.81 us) | 6.34 us (6.23 us - 6.47 us) |
| P4.1 | drop | 45.19 us (43.85 us - 45.36 us) | 45.68 us (44.16 us - 46.15 us) |
| P4.1 | retain | 46.63 us (46.18 us - 48.41 us) | 47.22 us (46.48 us - 47.72 us) |
| P5.1 | drop | 32.4 ns (32.3 ns - 38.9 ns) | 30.8 ns (29.8 ns - 32.3 ns) |
| P5.1 | retain | 33.5 ns (33.3 ns - 34.2 ns) | 33.2 ns (32.9 ns - 33.3 ns) |
| P5.2 | drop | 1.78 us (1.77 us - 1.86 us) | 1.79 us (1.78 us - 1.84 us) |
| P5.2 | retain | 1.79 us (1.78 us - 1.80 us) | 1.81 us (1.78 us - 1.82 us) |
| P5.3 | drop | 41.27 us (41.22 us - 48.55 us) | 44.27 us (41.41 us - 47.09 us) |
| P5.3 | retain | 44.28 us (41.83 us - 45.68 us) | 45.42 us (38.84 us - 45.79 us) |
| P5.4 | drop | 338.78 us (335.97 us - 347.74 us) | 336.92 us (333.45 us - 340.22 us) |
| P5.4 | retain | 334.56 us (334.06 us - 338.23 us) | 332.43 us (328.39 us - 338.21 us) |
| P6.1 | drop | 61.44 us (61.36 us - 61.72 us) | 61.03 us (60.62 us - 61.36 us) |
| P6.1 | retain | 61.29 us (61.28 us - 61.98 us) | 61.55 us (61.50 us - 61.57 us) |
| U-deep-u-repeated | drop | 431.8 ns (426.1 ns - 436.9 ns) | 431.7 ns (422.5 ns - 435.6 ns) |
| U-deep-u-repeated | retain | 459.9 ns (453.3 ns - 461.7 ns) | 463.5 ns (455.9 ns - 464.2 ns) |
| U-nested-before | drop | 82.3 ns (81.1 ns - 87.6 ns) | 84.4 ns (80.2 ns - 104.3 ns) |
| U-nested-before | retain | 91.0 ns (90.7 ns - 95.6 ns) | 88.2 ns (87.5 ns - 106.9 ns) |
| U-oneof-u-repeated | drop | 46.7 ns (45.5 ns - 60.1 ns) | 53.8 ns (53.4 ns - 70.3 ns) |
| U-oneof-u-repeated | retain | 49.0 ns (48.4 ns - 49.2 ns) | 56.0 ns (54.1 ns - 57.1 ns) |
| U-wire-DualResponse-left-as-wt5 | drop | 75.4 ns (74.5 ns - 76.0 ns) | 82.5 ns (81.6 ns - 84.0 ns) |
| U-wire-DualResponse-left-as-wt5 | retain | 84.1 ns (84.0 ns - 84.3 ns) | 86.0 ns (84.8 ns - 86.7 ns) |
| U-wire-ListMetricsResponse-batches-as-wt0 | drop | 606.3 ns (599.2 ns - 609.9 ns) | 601.7 ns (595.6 ns - 610.1 ns) |
| U-wire-ListMetricsResponse-batches-as-wt0 | retain | 613.8 ns (610.6 ns - 627.5 ns) | 614.0 ns (588.5 ns - 625.1 ns) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | drop | 399.9 ns (399.5 ns - 401.7 ns) | 398.8 ns (396.5 ns - 405.9 ns) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | retain | 415.4 ns (411.2 ns - 415.5 ns) | 410.6 ns (410.1 ns - 414.3 ns) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | drop | 32.3 ns (31.9 ns - 32.4 ns) | 30.4 ns (30.3 ns - 30.6 ns) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | retain | 36.7 ns (36.2 ns - 36.9 ns) | 35.5 ns (35.4 ns - 35.6 ns) |

## encode, end state transport-ready-tonic

| input | mode | explicit reset | reset on entry |
|---|---|---|---|
| P1.1 | drop | 428.8 ns (419.0 ns - 434.9 ns) | 430.7 ns (424.3 ns - 438.8 ns) |
| P1.1 | retain | 440.5 ns (422.2 ns - 441.3 ns) | 444.9 ns (427.4 ns - 447.1 ns) |
| P1.2 | drop | 86.28 us (86.02 us - 89.14 us) | 85.89 us (85.43 us - 88.72 us) |
| P1.2 | retain | 89.27 us (89.07 us - 89.69 us) | 90.27 us (89.51 us - 90.50 us) |
| P1.3 | drop | 3.97 us (3.96 us - 4.05 us) | 4.00 us (3.95 us - 4.02 us) |
| P1.3 | retain | 4.17 us (4.15 us - 4.21 us) | 4.18 us (4.08 us - 4.19 us) |
| P2.1 | drop | 628.8 ns (622.8 ns - 636.6 ns) | 614.2 ns (610.9 ns - 629.3 ns) |
| P2.1 | retain | 632.8 ns (631.1 ns - 644.3 ns) | 631.0 ns (627.2 ns - 644.0 ns) |
| P2.2 | drop | 302.13 us (297.70 us - 303.03 us) | 301.57 us (298.41 us - 307.02 us) |
| P2.2 | retain | 321.87 us (315.05 us - 326.72 us) | 323.03 us (321.94 us - 324.55 us) |
| P2.2/latin1 | drop | 304.55 us (303.90 us - 317.46 us) | 311.07 us (309.32 us - 312.33 us) |
| P2.2/latin1 | retain | 320.96 us (314.70 us - 323.49 us) | 321.08 us (320.46 us - 323.03 us) |
| P2.2/wide | drop | 316.38 us (311.76 us - 321.35 us) | 320.64 us (316.51 us - 336.76 us) |
| P2.2/wide | retain | 316.87 us (309.73 us - 328.28 us) | 320.51 us (319.31 us - 330.38 us) |
| P2.3 | drop | 165.09 us (162.41 us - 165.25 us) | 164.16 us (163.82 us - 165.63 us) |
| P2.3 | retain | 165.57 us (165.07 us - 170.27 us) | 168.63 us (166.70 us - 169.55 us) |
| P2.4 | drop | 236.69 us (236.27 us - 250.53 us) | 237.25 us (235.57 us - 251.75 us) |
| P2.4 | retain | 241.59 us (238.12 us - 242.91 us) | 241.43 us (240.44 us - 242.72 us) |
| P2.5 | drop | 8.87 us (8.83 us - 9.01 us) | 8.81 us (8.78 us - 9.05 us) |
| P2.5 | retain | 9.04 us (8.92 us - 9.35 us) | 8.96 us (8.90 us - 9.41 us) |
| P3.1 | drop | 6.49 us (6.43 us - 6.50 us) | 6.46 us (6.33 us - 6.55 us) |
| P3.1 | retain | 6.70 us (6.68 us - 6.90 us) | 6.57 us (6.41 us - 6.66 us) |
| P4.1 | drop | 45.60 us (44.95 us - 45.98 us) | 45.49 us (44.78 us - 46.33 us) |
| P4.1 | retain | 47.88 us (46.96 us - 47.89 us) | 48.32 us (46.57 us - 48.52 us) |
| P5.1 | drop | 176.2 ns (172.2 ns - 178.2 ns) | 174.2 ns (171.6 ns - 183.4 ns) |
| P5.1 | retain | 176.0 ns (174.7 ns - 180.7 ns) | 177.7 ns (175.5 ns - 186.8 ns) |
| P5.2 | drop | 1.98 us (1.96 us - 2.01 us) | 2.01 us (2.00 us - 2.02 us) |
| P5.2 | retain | 1.97 us (1.96 us - 1.97 us) | 1.97 us (1.95 us - 2.00 us) |
| P5.3 | drop | 56.90 us (56.19 us - 58.64 us) | 57.00 us (56.53 us - 58.88 us) |
| P5.3 | retain | 58.30 us (55.63 us - 58.50 us) | 56.36 us (55.58 us - 58.11 us) |
| P5.4 | drop | 342.29 us (339.70 us - 347.42 us) | 337.57 us (335.67 us - 346.74 us) |
| P5.4 | retain | 335.37 us (335.19 us - 341.15 us) | 335.92 us (334.62 us - 341.68 us) |
| P6.1 | drop | 62.04 us (61.58 us - 62.49 us) | 62.32 us (61.83 us - 62.96 us) |
| P6.1 | retain | 62.49 us (61.64 us - 62.55 us) | 62.07 us (61.87 us - 62.11 us) |
| U-deep-u-repeated | drop | 593.7 ns (581.3 ns - 594.6 ns) | 577.1 ns (570.7 ns - 594.9 ns) |
| U-deep-u-repeated | retain | 614.4 ns (606.2 ns - 618.9 ns) | 605.4 ns (598.8 ns - 609.8 ns) |
| U-nested-before | drop | 218.9 ns (217.7 ns - 224.9 ns) | 232.0 ns (228.2 ns - 232.9 ns) |
| U-nested-before | retain | 226.0 ns (222.9 ns - 240.5 ns) | 235.9 ns (231.8 ns - 254.2 ns) |
| U-oneof-u-repeated | drop | 192.0 ns (191.9 ns - 194.7 ns) | 201.5 ns (201.1 ns - 202.7 ns) |
| U-oneof-u-repeated | retain | 193.1 ns (192.5 ns - 200.4 ns) | 203.6 ns (202.9 ns - 210.7 ns) |
| U-wire-DualResponse-left-as-wt5 | drop | 211.4 ns (208.7 ns - 217.5 ns) | 230.8 ns (228.7 ns - 231.0 ns) |
| U-wire-DualResponse-left-as-wt5 | retain | 213.3 ns (212.7 ns - 213.7 ns) | 230.9 ns (230.3 ns - 233.7 ns) |
| U-wire-ListMetricsResponse-batches-as-wt0 | drop | 745.2 ns (743.4 ns - 752.5 ns) | 737.0 ns (736.6 ns - 757.5 ns) |
| U-wire-ListMetricsResponse-batches-as-wt0 | retain | 755.5 ns (749.3 ns - 764.2 ns) | 760.4 ns (759.4 ns - 761.1 ns) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | drop | 550.2 ns (550.1 ns - 559.2 ns) | 549.6 ns (545.5 ns - 562.0 ns) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | retain | 566.1 ns (560.6 ns - 569.5 ns) | 555.7 ns (553.1 ns - 560.7 ns) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | drop | 173.3 ns (173.0 ns - 174.2 ns) | 173.7 ns (173.3 ns - 175.5 ns) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | retain | 176.3 ns (175.8 ns - 176.8 ns) | 177.6 ns (177.4 ns - 177.9 ns) |

## the reset calls alone

| call | context | time |
|---|---|---|
| ak_enc_reset | every encode context of the rows above (50 cases x launches) | median 5.3 ns (5.2 ns - 5.5 ns) |
| ak_dec_reset_ListResultsResponse (the binding's options) | retain | 8.6 ns (8.4 ns - 8.6 ns) |
| ak_dec_reset_ListTasksDetailedResponse (the binding's options) | retain | 33.0 ns (33.0 ns - 33.6 ns) |
| ak_dec_reset_ListProbeResponse (the binding's options) | retain | 6.4 ns (6.4 ns - 6.6 ns) |
| ak_dec_reset_ListTaskSummaryResponse (the binding's options) | retain | 10.6 ns (10.4 ns - 10.8 ns) |
| ak_dec_reset_UploadResultDataMessage (the binding's options) | retain | 5.4 ns (5.4 ns - 5.5 ns) |
| ak_dec_reset_ListMetricsResponse (the binding's options) | retain | 5.7 ns (5.7 ns - 5.8 ns) |
| ak_dec_reset_DualResponse (the binding's options) | retain | 6.6 ns (6.6 ns - 6.7 ns) |
